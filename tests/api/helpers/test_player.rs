use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use super::test_game::{GameState, WsMessageIn, WsMessageOut};

pub struct TestPlayer {
    pub nickname: String,
    pub words: Vec<String>,
    pub tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl TestPlayer {
    pub async fn receive_game_state(&mut self) -> Result<GameState, String> {
        match self.rx.next().await {
            Some(Ok(message)) => {
                match serde_json::from_str(message.to_text().expect("Message was not a text")) {
                    Ok(WsMessageIn::GameState {
                        state,
                        players,
                        rounds,
                        amount_of_rounds,
                    }) => Ok(GameState {
                        state,
                        players,
                        rounds,
                        amount_of_rounds,
                    }),
                    Ok(WsMessageIn::Error {
                        r#type,
                        title,
                        detail,
                    }) => {
                        assert!(!title.is_empty());
                        assert!(!detail.is_empty());
                        Err(r#type)
                    }
                    Err(error) => Err(format!("Could not parse the message. Error: '{error}'.")),
                }
            }
            Some(Err(error)) => Err(format!("Websocket returned an error {error}")),
            None => Err("Websocket closed before expected.".to_string()),
        }
    }

    pub async fn start_game(&mut self, amount_of_rounds: i8) -> Result<GameState, String> {
        self.send_text_message(WsMessageOut::StartGame { amount_of_rounds })
            .await;
        self.receive_game_state().await
    }

    pub async fn send_words(&mut self) -> Result<GameState, String> {
        self.send_custom_words(self.words.clone()).await
    }

    pub async fn send_custom_words(&mut self, words: Vec<String>) -> Result<GameState, String> {
        self.send_text_message(WsMessageOut::PlayerWords { words })
            .await;
        self.receive_game_state().await
    }

    pub async fn send_voting_word(&mut self, word: Option<String>) -> Result<GameState, String> {
        self.send_text_message(WsMessageOut::PlayerVotingWord { word })
            .await;
        self.receive_game_state().await
    }

    pub async fn accept_players_voting_words(&mut self) -> Result<GameState, String> {
        self.send_text_message(WsMessageOut::AcceptPlayersVotingWords)
            .await;
        self.receive_game_state().await
    }

    pub async fn continue_to_next_round(&mut self) -> Result<GameState, String> {
        self.send_text_message(WsMessageOut::ContinueToNextRound)
            .await;
        self.receive_game_state().await
    }

    pub async fn play_again(&mut self) -> Result<GameState, String> {
        self.send_text_message(WsMessageOut::PlayAgain).await;
        self.receive_game_state().await
    }

    pub async fn send_raw_message(&mut self, message: Message) -> Result<GameState, String> {
        self.send_message(message).await;
        self.receive_game_state().await
    }

    pub async fn send_message(&mut self, message: Message) {
        self.tx.send(message).await.expect("Could not send message");
    }

    async fn send_text_message(&mut self, message: WsMessageOut) {
        self.send_message(Message::Text(
            serde_json::to_string(&message).expect("Could not serialize message"),
        ))
        .await;
    }
}
