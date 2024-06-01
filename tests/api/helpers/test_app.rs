use std::{net::SocketAddr, time::Duration};

use headcrab::config::Config;
use serde::Deserialize;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::helpers::test_game::GameFsmState;

use super::test_game::TestGame;

pub struct TestApp {
    pub base_address: String,
    pub inactivity_timeout: Duration,
}

impl TestApp {
    pub async fn spawn_app() -> TestApp {
        // Binding to port 0 triggers an OS scan for an available port, this way we can run tests in parallel where each runs its own application
        let random_port_address = SocketAddr::from(([0, 0, 0, 0], 0));
        let listener = TcpListener::bind(random_port_address)
            .await
            .expect("Failed to bind to bind random port.");
        let address = listener.local_addr().unwrap();
        std::env::set_var("ENVIRONMENT", "dev");
        let config = {
            let mut config = Config::get().expect("Failed to read configuration.");
            config.game.inactivity_timeout_seconds = 1;
            config
        };

        let server = headcrab::startup::create_web_server(config.clone(), listener);
        let _ = tokio::spawn(server);

        TestApp {
            base_address: format!("localhost:{}", address.port()),
            inactivity_timeout: config.game.inactivity_timeout(),
        }
    }

    pub async fn open_game_websocket(
        &self,
        game_id: &str,
        nickname: &str,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
        tokio_tungstenite::connect_async(format!(
            "ws://{}/game/{game_id}/player/{nickname}/ws",
            self.base_address
        ))
        .await
        .map(|websocket_stream| websocket_stream.0)
        .map_err(|error| format!("WebSocket could not be created. Error: '{error}'."))
    }

    pub async fn create_game_without_players() -> TestGame {
        let app = TestApp::spawn_app().await;

        let response = reqwest::Client::new()
            .post(format!("http://{}/game", app.base_address))
            .send()
            .await
            .expect("Failed to execute CreateGame request.");
        assert!(response.status().is_success());

        let game_created_response: GameCreatedResponse = response
            .json()
            .await
            .expect("Failed to parse GameCreatedResponse.");
        assert!(!game_created_response.id.is_empty());

        TestGame {
            app,
            id: game_created_response.id,
            players: vec![],
        }
    }

    pub async fn create_game(desired_state: GameFsmState) -> TestGame {
        let mut game = TestApp::create_game_without_players().await;

        let state = game.add_player("p1").await.unwrap();
        assert_eq!(state.state, GameFsmState::Lobby);
        assert_eq!(state.players.len(), 1);
        assert_eq!(state.players.get(0).unwrap().nickname, "p1");
        assert!(state.players.get(0).unwrap().is_host);

        // Make sure to read the events the other players receive when new players join, so that we leave a "clean" response channel for the tests
        let state = game.add_player("p2").await.unwrap();
        assert_eq!(state.state, GameFsmState::Lobby);
        assert_eq!(state.players.len(), 2);
        assert_eq!(state.players.get(0).unwrap().nickname, "p1");
        assert_eq!(state.players.get(1).unwrap().nickname, "p2");
        assert!(!state.players.get(1).unwrap().is_host);

        let state = game.add_player("p3").await.unwrap();
        assert_eq!(state.state, GameFsmState::Lobby);
        assert_eq!(state.players.len(), 3);
        assert_eq!(state.players.get(0).unwrap().nickname, "p1");
        assert_eq!(state.players.get(1).unwrap().nickname, "p2");
        assert_eq!(state.players.get(2).unwrap().nickname, "p3");
        assert!(!state.players.get(2).unwrap().is_host);

        let state = match desired_state {
            GameFsmState::Lobby => state.state,
            GameFsmState::PlayersSubmittingWords => {
                let _ = game.players[0]
                    .start_game(TestGame::AMOUNT_OF_ROUNDS)
                    .await
                    .unwrap();
                let _ = game.players[1].receive_game_state().await.unwrap();
                game.players[2].receive_game_state().await.unwrap().state
            }
            GameFsmState::PlayersSubmittingVotingWord => {
                game.players[0]
                    .start_game(TestGame::AMOUNT_OF_ROUNDS)
                    .await
                    .unwrap();

                game.players[1].receive_game_state().await.unwrap();
                game.players[2].receive_game_state().await.unwrap();

                game.players_send_words().await.state
            }
            GameFsmState::EndOfRound => todo!(),
            GameFsmState::EndOfGame => {
                let mut state = game.players[0].start_game(3).await.unwrap();
                for _ in 0..TestGame::AMOUNT_OF_ROUNDS {
                    let _ = game.complete_round().await;
                    state = game.continue_to_next_round().await;
                }
                state.state
            }
        };
        assert_eq!(state, desired_state);

        game
    }
}

#[derive(Deserialize)]
struct GameCreatedResponse {
    id: String,
}
