# WordNet Angular

Princeton WordNet Interface based on Angular.js and Rust, this is currently used to run three websites:

https://polylingwn.linguistic-lod.org/

http://wordnet-rdf.princeton.edu

https://en-word.net/

## Installation

The system is compiled and built with Cargo it can be built as usual with

    cargo build --release

The server is a single executable at `target/release/wordnet-angular`

Note: this project is dependent on [Rocket](http://rocket.rs) and needs the **nightly** build of Rust, see https://rocket.rs/v0.4/guide/quickstart/ for more details

## Usage

    USAGE:
        wordnet-angular [FLAGS] [OPTIONS]
    
    FLAGS:
        -h, --help       Prints help information
            --reload     Reload the indexes from the sources
        -V, --version    Prints version information
    
    OPTIONS:
        -p <port>                         The port to start the server on
        -s <princeton|polylingual|en>     The site design to use
            --wn <wn31.xml>               The WordNet file in GWC LMF-XML format, e.g., http://john.mccr.ae/wn31.xml.
                                          Default is data/wn31.xml
                                          

## Quick Start

To create an instance of http://en-word.net/ run the following commands

```sh
wget https://en-word.net/static/english-wordnet-2021.xml.gz
gunzip english-wordnet-2021.xml.gz
target/release/wordnet-angular --reload -s en --wn english-wordnet-2021.xml
```

If you get the following error then delete the file `wordnet.db` and try again

```
Failed to load WordNet: SQLite error: table synsets already exists
```
