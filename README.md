# WSTesting
Rust app for unit testing through websocket connections

## Running
cargo run -- (url) (glob)

## Example unit test
E:{"slideNumber":3,"songId":1,"type":"DisplayMessage","verseNumber":2}

S:aaa

E:{"reason":"malformed json","type":"BadRequestMessage"}

S:aaa

J:{"type":"BadRequestMessage"}

S:aaa

R:.*

##### E - expected
##### S - send
##### R - regex
##### J - json
