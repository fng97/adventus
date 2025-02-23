#!/usr/bin/env python

"""WebSocket echo server using the threading API."""

from websockets.sync.server import serve
import logging


logger = logging.getLogger(__name__)


def echo(websocket):
    logger.debug(f"Client at {websocket.remote_address} successfully connected")
    try:
        for message in websocket:
            logger.info(f"Received message: {message}")
            websocket.send(message)
    except Exception as e:
        logger.error(f"Error handling connection: {e}")


HOST = "localhost"
PORT = 8765


def main():
    logging.basicConfig(level=logging.DEBUG)
    logger.info(f"Launching WebSockets echo server at ws://{HOST}:{PORT}")
    with serve(echo, HOST, PORT, logger=logging.getLogger()) as server:
        server.serve_forever()


if __name__ == "__main__":
    main()
