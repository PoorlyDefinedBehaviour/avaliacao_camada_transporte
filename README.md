## Start the server

```
cargo r --bin server
```

## Start the client

```
cargo r --bin client -- --help

cargo r --bin client -- --username bob --room 1
cargo r --bin client -- --username john --room 1

# Force client to use a specific port.
cargo r --bin client -- --username john --room 1 --port 8888
```
