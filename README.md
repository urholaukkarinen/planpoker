# Planning Poker

Planning poker web application built with Yew, Actix Web & Websockets.

I started this project to learn web application development using Rust.	
The most important basic features have been implemented, but __UI is non-existent__.

You can create a room where others can join via a link. Users can place a vote,
and once everyone has voted, the admin can reveal the votes. An average value is calculated from the votes.

# Usage

```bash
# Start the backend
cd crates/backend
cargo run

# In another terminal instance, start the frontend
cd crates/frontend
trunk serve
```

