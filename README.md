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
        -s <princeton|polylingual>        The site design to use
            --wn <wn31.xml>               The WordNet file in GWC LMF-XML format, e.g., http://john.mccr.ae/wn31.xml.
                                          Default is data/wn31.xml
                                          
To create the database please run with the `--reload` flag.
