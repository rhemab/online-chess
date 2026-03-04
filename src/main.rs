use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use chess::{ChessMove, File, Game, Rank, Square};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppState {
    white: Option<Uuid>,
    black: Option<Uuid>,
    players_waiting: Vec<Uuid>,
    game: Game,
}

#[derive(Serialize, Deserialize, Debug)]
struct PlayerMove {
    source: String,
    target: String,
    piece: String,
}

#[tokio::main]
async fn main() {
    let app_state = AppState {
        white: None,
        black: None,
        players_waiting: vec![],
        game: Game::new(),
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
    // generate new id for new client
    let new_id = Uuid::new_v4();

    // set player color
    {
        let mut app_state = app_state.lock().await;

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
    }

    // send player color to client
    let msg = Message::Text(player_color.into());
    if let Err(err) = socket.send(msg).await {
        dbg!(err);
    }

    // recieve messages
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(bytes) => {
                if let Ok(player_move) = serde_json::from_str::<PlayerMove>(&bytes) {
                    println!("{:?}", player_move);

                    // create new chess move
                    let mut app_state = app_state.lock().await;
                    let source = square_from_str(&player_move.source);
                    let target = square_from_str(&player_move.target);

                    if let Some(source) = source
                        && let Some(target) = target
                    {
                        let new_move = ChessMove::new(source, target, None);
                        if app_state.game.make_move(new_move) {
                            let msg =
                                Message::Text(app_state.game.current_position().to_string().into());
                            if let Err(err) = socket.send(msg).await {
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
