# ELI - Electromagnetic Lookup Interface

Eli is an experimental system for observing and interpreting the wireless world.

The goal of Eli is simple in concept but large in scope:

> ingest a wide band of wireless signals to develop a real-world picture of what exists outside the house.

This project is not focused on a single protocol (like WiFi), a single device class, or a single radio band.  
Instead, Eli treats the electromagnetic environment as a continuous stream of physical information that can be sampled, buffered, analyzed, and reasoned about.



```zsh

#commands to run

cargo run -p elictl -- start rtl-00000001     

cargo run -p eli-router -- \                  
  --edge-device-bin ./target/debug/eli-device

```