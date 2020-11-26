This is a simple end-to-end test, publishing data via HTTP or MQTT, delivering with Kafka, to a Grafana dashboard.

## How to replicate

Take a look at the file [deploy/README.md](deploy/README.adoc). It guides you through the steps to replicate this.

## Building

### In a container

You will need:

* GNU Make
* A container engine (e.g. Docker or Podman)
* An internet connection

To build and publish, run:

    make CONTAINER_REGISTRY=quay.io/your-org
    
### In a container
To build locally :

* [Rust](https://rustup.rs)
* The following dependencies (packages names for fedora):
  - `gcc`
  - `openssl-devel`
  - `cyrus-sasl-devel`
  - `cmake`
  - `build-essential`
  - `libpq-devel`
* A container engine (e.g. Docker or Podman)
* An internet connection 

To build the binaries, run : 
     
     make cargo-build
     
Then build the docker images and publish them : 
     make images CONTAINER_REGISTRY=quay.io/your-org
