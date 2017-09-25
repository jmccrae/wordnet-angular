#!/bin/bash

mkdir -p data/

if [ ! -f data/wn31.xml ]
then
    curl http://john.mccr.ae/wn31.xml -o data/wn31.xml
fi

if [ ! -f data/ili-map-pwn30.tab ]
then
    curl https://github.com/globalwordnet/ili/raw/master/ili-map-pwn30.tab \
        -o data/ili-map-pwn30.tab
    curl https://github.com/globalwordnet/ili/raw/master/older-wn-mappings/ili-map-pwn15.tab \
        -o data/ili-map-pwn15.tab
    curl https://github.com/globalwordnet/ili/raw/master/older-wn-mappings/ili-map-pwn16.tab \
        -o data/ili-map-pwn16.tab
    curl https://github.com/globalwordnet/ili/raw/master/older-wn-mappings/ili-map-pwn17.tab \
        -o data/ili-map-pwn17.tab
    curl https://github.com/globalwordnet/ili/raw/master/older-wn-mappings/ili-map-pwn171.tab \
        -o data/ili-map-pwn171.tab
    curl https://github.com/globalwordnet/ili/raw/master/older-wn-mappings/ili-map-pwn20.tab \
        -o data/ili-map-pwn20.tab
    curl https://github.com/globalwordnet/ili/raw/master/older-wn-mappings/ili-map-pwn21.tab \
        -o data/ili-map-pwn21.tab
fi

if [ ! -f data/merged/adj.xml ]
then
    curl http://wordnetcode.princeton.edu/glosstag-files/WordNet-3.0-glosstag.tar.gz -o WordNet-3.0-glosstag.tar.gz
    tar xzvf WordNet-3.0-glosstag.tar.gz -C data
    mv data/WordNet-3.0/glosstag/merged/ data/
    rm -fr data/WordNet-3.0
    rm WordNet-3.0-glosstag.tar.gz
fi

if [ ! -f data/wns ]
then
    curl http://compling.hss.ntu.edu.sg/omw/all.zip -o all.zip
    unzip all.zip -d data
    rm all.zip
fi
