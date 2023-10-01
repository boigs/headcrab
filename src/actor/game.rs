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
    let (broadcast_channel, _): (
        broadcast::Sender<GameWideEvent>,
        broadcast::Receiver<GameWideEvent>,
    ) = broadcast::channel(32);

    while let Some(command) = rx.recv().await {
        match command {
            GameCommand::AddPlayer {
                player,
                player_actor,
            } => {
                match game.add_player(player) {
                    Err(_) => player_actor
                        .send(GameEvent::PlayerAlreadyExists)
                        .await
                        .unwrap(),
                    Ok(_) => {
                        player_actor
                            .send(GameEvent::PlayerAdded {
                                broadcast_channel: broadcast_channel.subscribe(),
                            })
                            .await
                            .unwrap();

                        broadcast_channel
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
                if broadcast_channel
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
