use crate::domain::{game::Game, player::Player};
use tokio::sync::{
    broadcast,
    mpsc::{Receiver, Sender},
};

pub enum GameCommand {
    AddPlayer {
        player: Player,
        player_actor: Sender<GameEvent>,
    },
    RemovePlayer {
        player: Player,
    },
}

pub enum GameEvent {
    PlayerAdded {
        broadcast_channel: broadcast::Receiver<GameWideEvent>,
    },
    PlayerAlreadyExists,
}

#[derive(Clone, Debug)]
pub enum GameWideEvent {
    GameState { players: Vec<Player> },
}

pub async fn handler(mut rx: Receiver<GameCommand>) {
    let mut game = Game::new();
    let (game_event_sender, _): (
        broadcast::Sender<GameWideEvent>,
        broadcast::Receiver<GameWideEvent>,
    ) = broadcast::channel(32);

    while let Some(command) = rx.recv().await {
        match command {
            GameCommand::AddPlayer {
                player,
                player_actor: response_channel,
            } => {
                match game.add_player(player) {
                    Err(_) => response_channel
                        .send(GameEvent::PlayerAlreadyExists)
                        .await
                        .unwrap(),
                    Ok(_) => {
                        response_channel
                            .send(GameEvent::PlayerAdded {
                                broadcast_channel: game_event_sender.subscribe(),
                            })
                            .await
                            .unwrap();

                        game_event_sender
                            .send(GameWideEvent::GameState {
                                players: Vec::from_iter(
                                    game.players().iter().map(|player| (*player).clone()),
                                ),
                            })
                            .unwrap();
                    }
                };
            }
            GameCommand::RemovePlayer { player } => {
                game.remove_player(&player.nickname);
                if game_event_sender
                    .send(GameWideEvent::GameState {
                        players: Vec::from_iter(
                            game.players().iter().map(|player| (*player).clone()),
                        ),
                    })
                    .is_err()
                {
                    println!("There are no player actors remaining listening to this game's broadcast messages. Closing game actor.");
                    return;
                }
            }
        }
    }
}
