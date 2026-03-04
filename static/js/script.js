const socketUrl = `ws://${window.location.host}/ws`;
const socket = new WebSocket(socketUrl);

let game_state = {
    position: "",
    turn: "",
    player_color: "",
}

// Connection opened
socket.addEventListener("open", (event) => {
    console.log("ws open");
    socket.send("Hello Server!");
});

// Listen for messages
socket.addEventListener("message", (event) => {
    console.log("Message from server:", event.data);
    let data = JSON.parse(event.data);

    // update game state
    game_state.position = data.position;
    game_state.turn = data.turn;
    game_state.player_color = data.player_color;

    var config = {
        draggable: true,
        showNotation: false,
        position: game_state.position,
        orientation: game_state.player_color,
        onDrop: onDrop
    }
    var board = Chessboard('board1', config)
});

socket.addEventListener("close", (event) => {
    console.log("ws close");
});

function onDrop (source, target, piece, newPos, oldPos) {
    console.log('Source: ' + source)
    console.log('Target: ' + target)
    console.log('Piece: ' + piece)
    console.log('New position: ' + Chessboard.objToFen(newPos))
    console.log('Old position: ' + Chessboard.objToFen(oldPos))
    console.log('~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~')

    let playerMove = {
        source,
        target,
        piece,
    };

    let jsonMsg = JSON.stringify(playerMove);
    socket.send(jsonMsg);
}

