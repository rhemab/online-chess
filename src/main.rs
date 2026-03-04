use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use chess::{ChessMove, Color, File, Game, Rank, Square};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tower_http::services::ServeDir;
use uuid::Uuid;

pub struct AppState {
    white: Option<Uuid>,
    black: Option<Uuid>,
    players_waiting: Vec<Uuid>,
    game: Game,
    sender: tokio::sync::broadcast::Sender<Broadcast>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PlayerMove {
    source: String,
    target: String,
    piece: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Broadcast {
    position: String,
    turn: String,
    player_color: String,
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel(16);

    let app_state = AppState {
        white: None,
        black: None,
        players_waiting: vec![],
        game: Game::new(),
        sender: tx,
    };

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(Arc::new(Mutex::new(app_state)))
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn websocket_handler(
    State(app_state): State<Arc<Mutex<AppState>>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Finalize upgrading the connection and call the provided callback with the stream.
    ws.on_failed_upgrade(|error| println!("Error upgrading websocket: {}", error))
        .on_upgrade(|socket| handle_socket(socket, app_state))
}

async fn handle_socket(mut socket: WebSocket, app_state: Arc<Mutex<AppState>>) {
    let player_color;

    // create initial broadcast for new connected client
    let mut broadcast = Broadcast {
        position: String::new(),
        turn: String::new(),
        player_color: String::new(),
    };

    // generate new id for new client
    let new_id = Uuid::new_v4();
    let mut rx;

    // set player color
    {
        let mut app_state = app_state.lock().await;
        rx = app_state.sender.subscribe();

        if app_state.white.is_none() {
            app_state.white = Some(new_id);
            player_color = "white";
        } else if app_state.black.is_none() {
            app_state.black = Some(new_id);
            player_color = "black";
        } else {
            app_state.players_waiting.push(new_id);
            player_color = "none";
        }

        broadcast.position = app_state.game.current_position().to_string();
        broadcast.turn = color_into_string(app_state.game.side_to_move());
        broadcast.player_color = player_color.to_string();
    }

    // send initial broadcast to new client
    if let Ok(json_msg) = serde_json::to_string(&broadcast) {
        if let Err(err) = socket.send(json_msg.into()).await {
            dbg!(err);
        }
    }

    let (mut ws_tx, mut ws_rx) = socket.split();

    // broadcast messages to all connected clients
    tokio::spawn(async move {
        while let Ok(mut msg) = rx.recv().await {
            // after receiving a msg on the channel
            // serialize and send the msg to clients
            msg.player_color = player_color.to_string();
            if let Ok(json_msg) = serde_json::to_string(&msg) {
                if let Err(err) = ws_tx.send(json_msg.into()).await {
                    dbg!(err);
                }
            }
        }
    });

    // recieve messages
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Text(bytes) => {
                if let Ok(player_move) = serde_json::from_str::<PlayerMove>(&bytes) {
                    println!("{:?}", player_move);

                    // create new chess move
                    let mut app_state = app_state.lock().await;
                    let source = square_from_str(&player_move.source);
                    let target = square_from_str(&player_move.target);

                    // if we have a source and a target, make the move
                    if let Some(source) = source
                        && let Some(target) = target
                    {
                        let new_move = ChessMove::new(source, target, None);
                        if app_state.game.make_move(new_move) {
                            // if move is legal, broadcast move
                            let msg = Broadcast {
                                position: app_state.game.current_position().to_string(),
                                turn: color_into_string(app_state.game.side_to_move()),
                                player_color: player_color.into(),
                            };
                            if let Err(err) = app_state.sender.send(msg) {
                                dbg!(err);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn color_into_string(color: Color) -> String {
    match color {
        Color::White => "white".into(),
        Color::Black => "black".into(),
    }
}

fn square_from_str(s: &str) -> Option<Square> {
    let mut chars = s.chars();
    let file = match chars.next()? {
        'a' => File::A,
        'b' => File::B,
        'c' => File::C,
        'd' => File::D,
        'e' => File::E,
        'f' => File::F,
        'g' => File::G,
        'h' => File::H,
        _ => return None,
    };
    let rank = match chars.next()? {
        '1' => Rank::First,
        '2' => Rank::Second,
        '3' => Rank::Third,
        '4' => Rank::Fourth,
        '5' => Rank::Fifth,
        '6' => Rank::Sixth,
        '7' => Rank::Seventh,
        '8' => Rank::Eighth,
        _ => return None,
    };
    Some(Square::make_square(rank, file))
}
