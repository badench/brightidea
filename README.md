# Bright Idea

## Overview

This project is a challenge from Brightidea. It is a chat room based off the source code located at https://github.com/seanmonstar/warp/blob/master/examples/websockets_chat.rs

The goal of this project is to modify the starting code to implement the following functionality

1. User should only receive messages sent to the same chat room. The core functionality is to use URL to identify different chat room (e.g. ws://localhost/chat/room1). Sending
   message to the same URL should only broadcast to other users connected to the same URL
2. Write transcript of each chat room to local file. design it in a way that would be able to scale for thousands of messages per second.
3. Add some basic support like users don’t need to clean up idle chat rooms etc.

## Running Code

To run the code first clone the repository. Then from top level directory execute `cargo run`. navigate to `http://localhost:3030` to interact with the server and join a chat room.

## Project structure

The src folder looks like the following

```
.
├── handlers
│   ├── chat_handlers.rs
│   └── mod.rs
├── main.rs
└── www
    └── index.html

```

main.rs contains the code to start the server and serve the accepted paths. Functions to handle requests are defined in the handlers director. The www directory is for web files.