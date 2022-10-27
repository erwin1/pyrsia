# Pyrsia p2p use cases

## Bootstrapping

```mermaid
sequenceDiagram;
participant node as Node
participant revproxy as boot.pyrsia.io
participant bootnode as Boot node X
participant otherbootnode as Boot node Y

node ->> revproxy: GET /status
revproxy ->> bootnode: GET /status from node X
revproxy ->> otherbootnode: or GET /status form node Y
revproxy ->> node: return status

node ->> node: select boot node X or Y

node ->> bootnode: dial
```


Questions:
- We seem to dial every 20s. How to improve?

## Requesting blocks at startup

```mermaid
sequenceDiagram;
participant node as Node
participant buildnode as Authorized<br>build node

node ->> buildnode: dialed and connected

node ->> buildnode: request new blocks

buildnode ->> node: return new blocks
```


## Receiving new blocks

### Current situation

```mermaid
sequenceDiagram;
participant node as Node
participant buildnodeX as Authorized<br>build node X
participant buildnodeY as Authorized<br>build node Y
participant othernode as Other Node

node ->> buildnodeX: dialed+connected
buildnodeY ->> buildnodeX: dialed+connected
othernode ->> buildnodeY: dialed+connected

buildnodeX ->> buildnodeX: publish new build

buildnodeX ->> node: send block(s)
node ->> node: store block(s)
buildnodeX ->> buildnodeY: send block(s)
buildnodeY ->> buildnodeY: store block(s)

note right of othernode: Other node<br>isn't notifified<br>of new block(s)
```

### Improved version (to be discussed)

```mermaid
sequenceDiagram;
participant node as Node
participant buildnodeX as Authorized<br>build node X
participant buildnodeY as Authorized<br>build node Y
participant othernode as Other Node

node ->> buildnodeX: dialed+connected
buildnodeY ->> buildnodeX: dialed+connected
othernode ->> buildnodeY: dialed+connected

buildnodeX ->> buildnodeX: publish new build

note left of buildnodeX: To be discussed<br> is it really necessary to<br>notify every node<br>immediately?

buildnodeX ->> node: send notify event
buildnodeX ->> buildnodeY: send notify event
buildnodeX ->> othernode: send notify event

node ->> buildnodeX: request new block(s)
buildnodeY ->> buildnodeX: request new block(s)
othernode ->> buildnodeX: request new block(s)

note right of othernode: Does this peer<br>retrieve new blocks from<br>node X or from node Y?
```

Questions:
- Is it really necessary (functionally) to send a notify event to all peers or could we wait until the peer triggers this itself (e.g. when it needs to look up an artifact)

- In the above scenario, would "Other node" retrieve new blocks from "build node X" or from "build node Y"?


## Retrieve artifact

```mermaid
sequenceDiagram;
participant node as Node
participant buildnode as Authorized<br>build node
participant buildnodeY as Another<br>Authorized<br>build node
participant dht as Distributed<br>Hash Table

note right of dht: The distributed<br>hash table is a logical<br>component that exists only<br>because it's shared accross peers

buildnodeY ->> buildnodeY: publish build
buildnodeY -->> dht: provide artifact file

node ->> buildnode: dialed+connected<br>+ received new blocks


node ->> node: lookup artifact<br>in transparency log

node -->> dht: lookup who provide artifact file

node ->> buildnodeY: request artifact file

buildnodeY ->> node: return artifact file
```


