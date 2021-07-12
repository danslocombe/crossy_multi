import("../pkg/index.js").catch(console.error);

var game_id = 1;
var player_name = "Dan";

fetch('/new')
.then(response => {console.log(response); return response})
.then(response => response.json())
.then(id => {
        console.log("Created game " + id);
	game_id = id;
	join();
});

function join() {
    fetch('/join?game_id=' + game_id + '&name=' + player_name)
        .then(response => response.json())
        .then(response => {
            init = true;
            console.log("Game ID : " + game_id);
            console.log(response);
            //printWords();
            //printState();
            connect_ws();
        });
}

function connect_ws() {
    let player_id = 1;
    var ws = new WebSocket("ws://localhost:8080/ws?game_id=" + game_id + '&player_id=' + player_id);
    //var ws = new WebSocket("ws://localhost:8080/ws?game_id=" + game_id);
    console.log("Opening ws");

    ws.onopen = () => {
        // Web Socket is connected, send data using send()
        ws.send("Message to send");
        console.log("Message is sent...");
    };

    ws.onmessage = evt => {
        var received_msg = evt.data;
        console.log("Message is received...");
        console.log(received_msg);
        printState();
    };

    ws.onclose = () => {
        // websocket is closed.
        console.log("Connection is closed..."); 
    };
}