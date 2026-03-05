const turn = document.getElementById("turn");
const resign = document.getElementById("resign");
const joinNewGame = document.getElementById("joinNewGame");
const topPlayer = document.getElementById("topPlayer");
const bottomPlayer = document.getElementById("bottomPlayer");
const editUsername = document.getElementById("editUsername");

// load saved name on page load
const savedName = localStorage.getItem('playerName');
if (savedName) bottomPlayer.textContent = savedName;
let nameSent = false;

const host = window.location.host;
let socketUrl = `wss://${host}/ws`;
if (window.location.protocol == "http:") {
    socketUrl = `ws://${host}/ws`;
}
const socket = new WebSocket(socketUrl);

let game_state = {
    position: "",
    turn: "",
    player_color: "none",
    white_name: "",
    black_name: "",
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
    console.log(data);

    // update game state
    game_state.position = data.position;
    game_state.turn = data.turn;
    game_state.player_color = data.player_color;
    game_state.white_name = data.white_name;
    game_state.black_name = data.black_name;

    // set player names
    if (data.player_color == "black") {
        topPlayer.textContent = data.white_name;
        bottomPlayer.textContent = data.black_name;
    } else {
        topPlayer.textContent = data.black_name;
        bottomPlayer.textContent = data.white_name;
    }

    // if playing, send saved name
    if (!nameSent && data.player_color != "none") {
        if (savedName) {
            // send the name to the server
            let playerName = {
                player_color: game_state.player_color,
                player_name: savedName,
            };
            let jsonMsg = JSON.stringify(playerName);
            socket.send(jsonMsg);
            nameSent = true;
        }
    }

    // set the board position
    board.position(game_state.position, false);
    board.orientation(game_state.player_color);

    // show when it's your turn
    if (game_state.turn == game_state.player_color) {
        // it's your turn
        turn.innerHTML = "Your turn";
        resign.classList.remove("invisible");
        editUsername.classList.remove("invisible");
    } else if (game_state.player_color == "none") {
        // you're a spectator
        turn.innerHTML = `${game_state.turn}'s move`;
        resign.classList.add("invisible");
    } else {
        // it's not your turn
        turn.innerHTML = "";
        resign.classList.remove("invisible");
        editUsername.classList.remove("invisible");
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
            player_resign: game_state.player_color,
        };
        let jsonMsg = JSON.stringify(playerResign);
        socket.send(jsonMsg);
    }
});

joinNewGame.addEventListener("click", (event) => {
    window.location.reload();
});

editUsername.addEventListener("click", (event) => {
    const input = document.createElement('input');
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            bottomPlayer.textContent = input.value;
            localStorage.setItem('playerName', input.value);
            input.replaceWith(bottomPlayer);
            editUsername.classList.remove("invisible");

            // send the name to the server
            let playerName = {
                player_color: game_state.player_color,
                player_name: input.value,
            };
            let jsonMsg = JSON.stringify(playerName);
            socket.send(jsonMsg);
        }
    });
    input.value = bottomPlayer.textContent;
    bottomPlayer.replaceWith(input);
    input.focus();
    editUsername.classList.add("invisible");
});
