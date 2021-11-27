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

The Project has the following directory structure.

```
.
├── Cargo.lock
├── Cargo.toml
├── README.md
├── logs
│   ├── room1.log
│   └── room2.log
└── src
    ├── handlers
    │   ├── chat_handlers.rs
    │   └── mod.rs
    ├── logger.rs
    ├── main.rs
    └── www
        └── index.html


```

### src

The src folder contains all the source code for the binary application to run. The handlers module contains code for handling a request at a specific route. The file logger.rs
contains the logging code. This could eventually be moved to it's own module similar to handlers when more code is added that makes sense to group. The www directory contains web
files to serve. To run the application use the command
`cargo run` from the room directory.

### logs

This directory contains chat logs for each room connected to. Two example logs that were written concurrently can be found here. log files are in the format <timestamp>\t<
user+message>\n

### Testing

So far logger is the only module with testing. To run tests use the command `cargo test`