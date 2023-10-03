use crate::domain::{game::Game, player::Player};
use tokio::sync::broadcast::error::SendError;
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
                match game.add_player(player.clone()) {
                    Err(_) => {
                        if player_actor
                            .send(GameEvent::PlayerAlreadyExists)
                            .await
                            .is_err()
                        {
                            println!("ERROR: Sent GameEvent::PlayerAlreadyExists to Player but the channel is closed.");
                        }
                    }
                    Ok(_) => {
                        if player_actor
                            .send(GameEvent::PlayerAdded {
                                broadcast_channel: broadcast_channel.subscribe(),
                            })
                            .await
                            .is_err()
                        {
                            println!("ERROR: Sent GameEvent::PlayerAdded to Player but the channel is closed. Removing the Player.");
                            game.remove_player(&player.nickname);
                        } else if send_game_state(&broadcast_channel, &game).is_err() {
                            println!("ERROR: Sent GameWideEvent::GameState to Broadcast but the channel is closed. Stopping the Game.");
                            return;
                        };
                    }
                };
            }
            GameCommand::RemovePlayer { player } => {
                game.remove_player(&player.nickname);
                if game.players().is_empty() {
                    println!(
                        "INFO: Removed Player from the Game, no more Players, stopping the Game."
                    );
                    return;
                }
                if send_game_state(&broadcast_channel, &game).is_err() {
                    println!("ERROR: There are no Players remaining listening to this game's broadcast messages but there are player objects in the game. Stopping the Game.");
                    return;
                }
            }
        }
    }
}

fn send_game_state(
    broadcast: &broadcast::Sender<GameWideEvent>,
    game: &Game,
) -> Result<usize, SendError<GameWideEvent>> {
    broadcast.send(GameWideEvent::GameState {
        players: Vec::from_iter(game.players().iter().map(|player| (*player).clone())),
    })
}
