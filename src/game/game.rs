use crate::game::entity::card::Card;
use crate::game::entity::player::{Player, PlayerView};
use crate::game::game_state::GameState;
use crate::game::lua_context::LuaContext;
use crate::game::script_manager::ScriptManager;
use crate::logger;
use crate::models::client_requests::PlayCardRequest;
use crate::models::init_server::PreloadPlayer;
use crate::tcp::client::Client;
use crate::utils::errors::{GameInstanceError, GameLogicError};
use crate::utils::logger::Logger;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GameInstance {
    pub game_state: Arc<RwLock<GameState>>, // The current game state, shared across tasks.
    pub script_manager: Arc<RwLock<ScriptManager>>, // The Lua script manager for handling game logic scripts.
    pub full_cards: Arc<RwLock<HashMap<String, Card>>>,
    pub connected_players: Arc<RwLock<HashMap<String, Arc<RwLock<Player>>>>>,
}

impl GameInstance {
    pub async fn create_instance(players: Vec<PreloadPlayer>) -> Result<Self, GameInstanceError> {
        let mut lua_vm = ScriptManager::new_vm();
        lua_vm
            .load_scripts()
            .map_err(|e| GameInstanceError::PlaceHolderError)?;
        lua_vm.set_globals().await;
        let scripts = Arc::new(RwLock::new(lua_vm));
        //

        let mut full_cards_map: HashMap<String, Card> = HashMap::new();
        let mut connected_players: HashMap<String, Arc<RwLock<Player>>> = HashMap::new();
        let mut connect_players_views: HashMap<String, Arc<RwLock<PlayerView>>> = HashMap::new();

        for player in &players {
            let player_profile = Player::preload_player_profile(&player.id)
                .await
                .map_err(|e| GameInstanceError::PlaceHolderError)?;

            let player_deck = Player::preload_player_deck(&player.deck_id)
                .await
                .map_err(|e| GameInstanceError::PlaceHolderError)?;

            let full_cards = Card::request_cards(&player_deck.cards)
                .await
                .map_err(|e| GameInstanceError::PlaceHolderError)?;

            for card in full_cards {
                full_cards_map.insert(card.id.clone(), card);
            }

            let deck_view = player_deck.create_view(&full_cards_map, &player_profile.id);
            let player_view = Arc::new(RwLock::new(PlayerView::from_player(
                &player_profile.id,
                player_deck.cards.len(),
            )));
            
            let player = Player::preload_player(player_profile, player_deck, deck_view, player_view.clone()).await;

            connect_players_views.insert(player.id.clone(), player_view);
            connected_players.insert(player.id.clone(), Arc::new(RwLock::new(player)));
        }

        Ok(Self {
            script_manager: scripts,
            full_cards: Arc::new(RwLock::new(full_cards_map)),
            connected_players: Arc::new(RwLock::new(connected_players)),
            game_state: Arc::new(RwLock::new(GameState::new_game(connect_players_views))),
        })
    }
}

// Player Actions
impl GameInstance {
    pub async fn play_card(
        self: Arc<Self>,
        client: Arc<Client>,
        request: &PlayCardRequest,
    ) -> Result<(), GameLogicError> {
        let game_state = self.game_state.read().await;
        let player_views = game_state.player_views.read().await;

        // Clone and lock the Client player object to compare identity and access full player data.
        let player_clone = Arc::clone(&client.player);
        let player_guard = player_clone.read().await;

        // Try to fetch the PrivatePlayerView for the given player ID. Return an error if not found.
        let player_view = player_views.get(&request.actor_id).ok_or_else(|| {
            logger!(DEBUG, "[PLAY CARD] Play card actor: {}", &request.actor_id);
            logger!(DEBUG, "[PLAY CARD] Play card client: {}", &player_guard.id);
            return GameLogicError::PlayerNotFound;
        })?;

        let player_view_clone = Arc::clone(player_view);
        let player_view_guard = player_view_clone.read().await;

        // Ensure that the client attempting the action matches the player in the request.
        if &player_guard.id != &player_view_guard.id {
            return Err(GameLogicError::PlayerIdDoesNotMatch);
        }

        //Confirm it is currently this player's turn.
        if &player_view_guard.id != &request.actor_id {
            return Err(GameLogicError::NotPlayerTurn);
        }

        // Verifies if the card played is actually in the player's hand. This does not account for
        // out-of-hand plays from special interactions as they do not exist yet.
        let player_hand = player_view_guard.current_hand.iter();
        let card_view = player_hand
            .flatten()
            .find(|c| c.id == request.card_id)
            .ok_or_else(|| GameLogicError::CardPlayedIsNotInHand)?;

        // Verify that the requested card is in the player's current hand.
        // Retrieve the full card details from game_cards. If not present, fetch it from external storage and add it to the shared card list.
        let game_cards_lock = self.full_cards.read().await;
        let full_card = match game_cards_lock.get(&card_view.id) {
            Some(card) => card,
            None => {
                let card = Card::request_card(&card_view.id)
                    .await
                    .map_err(|_| GameLogicError::UnableToGetCardDetails)?;
                self.add_card(card).await;
                game_cards_lock.get(&card_view.id).ok_or_else(|| {
                    return GameLogicError::UnableToGetCardDetails;
                })?
            }
        };

        // Iterate over the card’s on_play triggers, creating a Lua execution context for each.
        for action in &full_card.on_play {
            let lua_context = LuaContext::new(
                Arc::clone(&self.game_state),
                card_view,
                None,
                "on_play".to_string(),
                action.to_string(),
            )
            .await;

            // Execute each script action using the ScriptManager and apply the resulting game actions to the state.
            let script_manager_guard = self.script_manager.read().await;
            let game_actions = script_manager_guard
                .call_function_ctx(action, lua_context)
                .await?;

            game_state.apply_actions(game_actions).await;
        }

        Ok(())
    }
}

// Card implementations
impl GameInstance {
    /// Store a card in the game state.
    pub async fn add_card(&self, card: Card) {
        let mut card_vec = self.full_cards.write().await;
        card_vec.insert(card.id.to_string(), card);
    }
}

// Player implementations
impl GameInstance {
    // pub async fn add_player(&mut self, player: Arc<Player>) {
    //     let player_view = PlayerView::from_player(player.clone());
    //     let player_view_guard = Arc::new(RwLock::new(player_view));
    //     let mut game_state_guard = self.game_state.write().await;
    //
    //     if game_state_guard.blue_player.is_empty() {
    //         game_state_guard.blue_player = player.id.clone();
    //     } else if game_state_guard.red_player.is_empty() {
    //         game_state_guard.red_player = player.id.clone();
    //     } else {
    //         logger!(WARN, "[GAME STATE] Both players are already connected");
    //         return;
    //     }
    //
    //     let mut player_views_guard = game_state_guard.player_views.write().await;
    //     player_views_guard.insert(player.id.clone(), player_view_guard);
    // }
}
