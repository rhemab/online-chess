const socketUrl = `ws://${window.location.host}/ws`;
const socket = new WebSocket(socketUrl);

// Connection opened
socket.addEventListener("open", (event) => {
    console.log("ws open");
    socket.send("Hello Server!");
});

// Listen for messages
socket.addEventListener("message", (event) => {
    console.log("Message from server:", event.data);
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

var config = {
    draggable: true,
    showNotation: false,
    position: 'start',
    onDrop: onDrop
}
var board = Chessboard('board1', config)
