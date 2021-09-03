cargo run --bin wordnet-rdf-dump -- -s en | rapper -e -i turtle -o ntriples -I file:zxy - | sort -u | rapper -i ntriples -o turtle -I file:zxy -f 'xmlns:dc="http://purl.org/dc/terms/"' -f 'xmlns:ili="http://ili.globalwordnet.org/ili/"' -f 'xmlns:lime="http://www.w3.org/ns/lemon/lime#"' -f 'xmlns:ontolex="http://www.w3.org/ns/lemon/ontolex#"' -f 'xmlns:owl="http://www.w3.org/2002/07/owl#"' -f 'xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"' -f 'xmlns:schema="http://schema.org/"' -f 'xmlns:skos="http://www.w3.org/2004/02/skos/core#"' -f 'xmlns:synsem="http://www.w3.org/ns/lemon/synsem#"' -f 'xmlns:wn="https://globalwordnet.github.io/schemas/wn#"' -f 'xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"' -f 'xmlns:dct="http://purl.org/dc/terms/"' -f 'xmlns:oewnlemma="https://en-word.net/lemma/"' -f 'xmlns:oewnid="https://en-word.net/id/"' - |gzip > english-wordnet-2021.ttl.gz