const turn = document.getElementById("turn");
const resign = document.getElementById("resign");
const joinNewGame = document.getElementById("joinNewGame");

const socketUrl = `wss://${window.location.host}/ws`;
const socket = new WebSocket(socketUrl);

let game_state = {
    position: "",
    turn: "",
    player_color: "none",
}

function onDrop (source, target, piece) {
    if (game_state.turn != game_state.player_color) {
        return "snapback";
    }

    let playerMove = {
        source,
        target,
        piece,
    };

    let jsonMsg = JSON.stringify(playerMove);
    socket.send(jsonMsg);
}

var config = {
    draggable: true,
    showNotation: false,
    position: game_state.position,
    orientation: game_state.player_color,
    onDrop: onDrop,
    pieceTheme: 'img/chesspieces/lichess/{piece}.svg',
}
var board = Chessboard('board1', config)
$(window).resize(board.resize);

// Connection opened
socket.addEventListener("open", (event) => {
    console.log("ws open");
});

// Listen for messages
socket.addEventListener("message", (event) => {
    let data = JSON.parse(event.data);

    // update game state
    game_state.position = data.position;
    game_state.turn = data.turn;
    game_state.player_color = data.player_color;

    // set the board position
    board.position(game_state.position, false);
    board.orientation(game_state.player_color);

    // show when it's your turn
    if (game_state.turn == game_state.player_color) {
        turn.innerHTML = "Your turn";
        resign.classList.remove("invisible");
    } else if (game_state.player_color == "none") {
        turn.innerHTML = `${game_state.turn}'s move`;
        resign.classList.add("invisible");
    } else {
        turn.innerHTML = "";
        resign.classList.remove("invisible");
    }

    // check for game over
    if (data.game_result.length > 0) {
        turn.innerHTML = data.game_result;
        joinNewGame.classList.remove("invisible");
        socket.close();
    }
});

socket.addEventListener("close", (event) => {
    console.log("ws close");
});

resign.addEventListener("click", (event) => {
    if (window.confirm("Do you want to resign the game?")) {
        console.log("resigned");
        let playerResign = {
            player_color: game_state.player_color,
        };
        let jsonMsg = JSON.stringify(playerResign);
        socket.send(jsonMsg);
    }
});

joinNewGame.addEventListener("click", (event) => {
    window.location.reload();
});
