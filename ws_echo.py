#!/usr/bin/env python

"""Echo server using the threading API."""

from websockets.sync.server import serve


def echo(websocket):
    for message in websocket:
        websocket.send(message)


HOST = "localhost"
PORT = 8765


def main():
    print(f"WebSockets echo server at {HOST}:{PORT}")
    with serve(echo, HOST, PORT) as server:
        server.serve_forever()


if __name__ == "__main__":
    main()
