use axum::extract::ws::{Message, WebSocket};
use tokio::{
    select,
    sync::mpsc::{self, Receiver, Sender},
};

use crate::actor::game::{
    GameCommand::{self, *},
    GameEvent,
};
use crate::domain::player::Player;

use crate::actor::game::GameWideEvent;

pub async fn handler(mut socket: WebSocket, nickname: String, game_channel: Sender<GameCommand>) {
    let (tx, mut rx): (Sender<GameEvent>, Receiver<GameEvent>) = mpsc::channel(32);

    game_channel
        .send(AddPlayer {
            player: Player::new(&nickname),
            response_channel: tx,
        })
        .await
        .unwrap();

    let mut broadcast_receiver = match rx.recv().await {
        Some(GameEvent::PlayerAdded { broadcast_channel }) => broadcast_channel,
        Some(GameEvent::PlayerAlreadyExists) => {
            socket
                .send(Message::Text(
                    "Loco que ya existe un jugador con ese nombre".to_string(),
                ))
                .await
                .unwrap();
            socket.close().await.unwrap();
            return;
        }
        _ => panic!("Channel closed or something"), // TODO @sergi
    };

    loop {
        select! {
            game_wide_message = broadcast_receiver.recv() => {
                match game_wide_message {
                    Ok(GameWideEvent::PlayerList { players }) => socket.send(Message::Text(serde_json::to_string(&players).unwrap())).await.unwrap(),
                    Err(_) => panic!("aaaaaaaa"),
                }
            },
            _socket_message = socket.recv() => {

            },
            _game_event = rx.recv() => {

            },
        }
    }
}
