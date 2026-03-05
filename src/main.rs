use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use chess::{ChessMove, Color, File, Game, GameResult, Rank, Square};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tower_http::services::ServeDir;
use uuid::Uuid;

pub struct AppState {
    white: Option<Uuid>,
    black: Option<Uuid>,
    players_waiting: VecDeque<Uuid>,
    game: Game,
    sender: tokio::sync::broadcast::Sender<Broadcast>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PlayerMove {
    source: String,
    target: String,
    piece: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PlayerResign {
    player_color: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Broadcast {
    position: String,
    turn: String,
    player_color: String,
    game_result: String,
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel(16);

    let app_state = AppState {
        white: None,
        black: None,
        players_waiting: VecDeque::new(),
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
    let mut player_color;

    // create initial broadcast for new connected client
    let mut broadcast = Broadcast::default();

    // generate new id for new client
    let player_id = Uuid::new_v4();
    let mut rx;

    // set player color
    {
        let mut app_state = app_state.lock().await;
        rx = app_state.sender.subscribe();

        if app_state.white.is_none() {
            app_state.white = Some(player_id);
            player_color = "white";
        } else if app_state.black.is_none() {
            app_state.black = Some(player_id);
            player_color = "black";
        } else {
            app_state.players_waiting.push_back(player_id);
            player_color = "none";
        }

        broadcast.position = app_state.game.current_position().to_string();
        broadcast.turn = color_into_string(app_state.game.side_to_move());
        broadcast.player_color = player_color.to_string();
        if let Some(result) = app_state.game.result() {
            broadcast.game_result = result_to_string(result);
        }
    }

    // send initial broadcast to new client
    if let Ok(json_msg) = serde_json::to_string(&broadcast) {
        if let Err(err) = socket.send(json_msg.into()).await {
            dbg!(err);
        }
    }

    let (mut ws_tx, mut ws_rx) = socket.split();
    let app_state_clone = app_state.clone();

    // broadcast messages to all connected clients
    tokio::spawn(async move {
        while let Ok(mut msg) = rx.recv().await {
            let app_state = app_state_clone.lock().await;
            if app_state.white == Some(player_id) {
                player_color = "white";
            }
            if app_state.black == Some(player_id) {
                player_color = "black";
            }
            // after receiving a msg on the channel
            // serialize and send the msg to clients
            msg.player_color = player_color.to_string();
            if let Ok(json_msg) = serde_json::to_string(&msg) {
                if let Err(err) = ws_tx.send(json_msg.into()).await {
                    dbg!(err);
                    return;
                }
            }
        }
    });

    // recieve messages
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Text(bytes) => {
                if let Ok(player_move) = serde_json::from_str::<PlayerMove>(&bytes) {
                    // lock the app_state
                    let mut app_state = app_state.lock().await;

                    // create new chess move
                    let source = square_from_str(&player_move.source);
                    let target = square_from_str(&player_move.target);

                    // if we have a source and a target, make the move
                    if let Some(source) = source
                        && let Some(target) = target
                    {
                        let new_move = ChessMove::new(source, target, None);
                        app_state.game.make_move(new_move);
                    }

                    let mut broadcast = Broadcast::default();
                    broadcast.position = app_state.game.current_position().to_string();
                    broadcast.turn = color_into_string(app_state.game.side_to_move());
                    if let Some(result) = app_state.game.result() {
                        broadcast.game_result = result_to_string(result);
                    }
                    if let Err(err) = app_state.sender.send(broadcast) {
                        dbg!(err);
                    }
                } else if let Ok(player_resign) = serde_json::from_str::<PlayerResign>(&bytes) {
                    let mut app_state = app_state.lock().await;
                    if let Some(color) = string_into_color(&player_resign.player_color) {
                        app_state.game.resign(color);

                        let mut broadcast = Broadcast::default();
                        broadcast.position = app_state.game.current_position().to_string();
                        broadcast.turn = color_into_string(app_state.game.side_to_move());
                        if let Some(result) = app_state.game.result() {
                            broadcast.game_result = result_to_string(result);
                        }
                        if let Err(err) = app_state.sender.send(broadcast) {
                            dbg!(err);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // if player disconnects, remove from game state
    let mut app_state = app_state.lock().await;
    if app_state.white == Some(player_id) {
        app_state.white = app_state.players_waiting.pop_front();
        app_state.game = Game::new();
    } else if app_state.black == Some(player_id) {
        app_state.black = app_state.players_waiting.pop_front();
        app_state.game = Game::new();
    } else if app_state.players_waiting.contains(&player_id) {
        app_state.players_waiting.retain(|item| *item != player_id);
    }

    // send broadcast to so that clients are updated
    let mut broadcast = Broadcast::default();
    broadcast.position = app_state.game.current_position().to_string();
    broadcast.turn = color_into_string(app_state.game.side_to_move());
    if let Some(result) = app_state.game.result() {
        broadcast.game_result = result_to_string(result);
    }
    if let Err(err) = app_state.sender.send(broadcast) {
        dbg!(err);
    }
}

fn result_to_string(result: GameResult) -> String {
    match result {
        GameResult::WhiteCheckmates => "White wins by checkmate".to_string(),
        GameResult::WhiteResigns => "White resigns, black wins".to_string(),
        GameResult::BlackCheckmates => "Black wins by checkmate".to_string(),
        GameResult::BlackResigns => "Black resigns, white wins".to_string(),
        GameResult::Stalemate => "Draw by stalemate".to_string(),
        GameResult::DrawAccepted => "Draw by agreement".to_string(),
        GameResult::DrawDeclared => "Draw declared".to_string(),
    }
}

fn string_into_color(color: &str) -> Option<Color> {
    match color {
        "white" => Some(Color::White),
        "black" => Some(Color::Black),
        _ => None,
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
