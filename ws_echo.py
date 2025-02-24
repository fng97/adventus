#!/usr/bin/env python

"""WebSocket echo server using the threading API."""

from websockets.sync.server import serve
import logging


def echo(websocket):
    for message in websocket:
        websocket.send(message)


HOST = "localhost"
PORT = 8765


def main():
    logging.basicConfig(level=logging.DEBUG)
    print(f"Launching WebSockets echo server at ws://{HOST}:{PORT}\n")
    with serve(echo, HOST, PORT, logger=logging.getLogger()) as server:
        server.serve_forever()


if __name__ == "__main__":
    main()
