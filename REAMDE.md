# tweet_analyzer
A web app that analyzes and correlates the speech patterns of Twitter users.

## Methodology

### Terms

* [Twitter](https://twitter.com/home): A social media platform that focuses on short posts.
* Document: The set of all obtainable tweets of a given user concatenated, and with special characters removed.
* [w-shingling](https://en.wikipedia.org/wiki/W-shingling): The process of dividing a document into `w` size words.  E.g., the 2-grams of "the quick brown fox" are "the quick", "quick brown", and "brown fox".
* [Jaccard Similarity](https://en.wikipedia.org/wiki/Jaccard_index): A similarity index between two documents computed by taking the intersection over the union of each document's set of w-shingles.
* [MinHash](https://en.wikipedia.org/wiki/MinHash): A technique for reducing a set into a hash by hashing every value and taking only the minimum hash value.
* [Locality Sensitive Hashing](https://en.wikipedia.org/wiki/Locality-sensitive_hashing): The process of reducing a set of w-shingles for a document into a signature that can be used to _roughly approximate_ the Jaccard Similarity between two documents.  This is done by choosing a set of randomly generated universal hash functions and applying the MinHash for each document for each universal hash function.  Useful when working with a lot of documents with large w-shingle sets.

### Server

The server is a [Rust](https://www.rust-lang.org/) application that uses [Rocket](https://rocket.rs/) as a web server and [Tokio](https://tokio.rs/) as an async runtime.

Besides starting the [web server](server/src/web.rs), the [entrypoint](server/src/main.rs) spawns each "stage" of the analysis into its own thread/channel pair.  The `tweet_grabber` sends messages to the `tweet_analyzer`, which, in turn, sends messages to the `similarity_computer`, all the while using a [mongoDB](https://www.mongodb.com/) as a backing store for the processing at each stage.  Each of the stages are discussed below.

#### Tweet Grabber

The [tweet grabber](server/src/tweet_grabber.rs) accepts messages (in the form of a user handle as a string) on its receive channel, and spawns a new tasks designed to
1. Pull tweets from the twitter API.
2. Wait when the API limit has been exhausted.
3. Polish up the text by lower casing it and removing special characters.
4. Insert the tweets into the database.
5. Push a message on its send channel indicating that this user handle is ready for analysis.

#### Tweet Analyzer

The [tweet analyzer](server/src/tweet_analyzer.rs) accepts messages (in the form of a user handle as a string) on its receive channel, and spawns a new tasks designed to
1. Pull tweets from the database of polished tweets.
2. Compute and aggregate the w-shingles for the set of tweets as a whole.
3. Insert the shingle data into the database.
4. Compute a signature for the set of shingles (optionally with bounds on `w` and bounds on the number of time the shingle was "seen").
5. Insert the signature into the database.
6. Push a message on its send channel indicating that this user handle is ready for comparisons.

#### Similarity Computer

The [tweet analyzer](server/src/similarity_computer.rs) accepts messages (in the form of a user handle as a string) on its receive channel, and spawns a new tasks designed to
1. Pull all signatures from the database.
2. Calculate the approximate Jaccard Similarity of two users by performing an element-by-element equality comparison between each signature.
3. Insert the newly computed similarities into the database.
4. Push a message on its send channel indicating that this user handle's analysis is complete.

### Assertions

The assertions of this process are that the "similarities" computed by the end of this process are approximately equal to the Jaccard Similarity between the two user's tweets as a body of w-shingles.  The process works as follows.

First, a user's tweets are downloaded, lowercased, stripped of special characters, and merged to yield a **document**.  This document, for the purposes of discussing this process is equatable to the "user".  A "user" is a "document", and vice versa.

Then, the document is computed into a set of **w-shingles**.  The size of the shingles is determined by the settings.  The shingles are placed into frequency bins, and the most frequently used shingles are preferred over later stages of the process (the degree to which is also determined by settings).

Each document's set of shingles, `D_d`, is then minhashed for each item of a set of random universal hash functions, `H_y` (see [mhs](server/src/mhs.rs)).  Each document as a set of shingles (e.g., `D_0`, `D_1`, etc.) is minhashed by each hash function (e.g., `H_0`, `H_1`, etc.) yielding a signature for each document, `S_d`.  This signature looks like this

```
S_d = [ min(H_0(D_d)), min(H_1(D_d)), ..., min(H_N(D_d)) ]
```

for every document, `d`, in `D`, where `N` is the _signature length_.

As stated above, comparing the signatures of two documents (e.g., `D_40` and `D_45`) is _approximately equivalent_ to the Jaccard Similarity between the w-shingle set of each of those documents.  This assertion forms the basis for the entire analysis.

### Client

The client is a [d3](https://d3js.org/) driven, [LitElement](https://lit-element.polymer-project.org/) based thin client primarily authored with [TypeScript](https://www.typescriptlang.org/).  The meat is in the [main page](client/src/main-page.ts).

Each Twitter user is represented as a node, and each similarity is represented as a link.  There are four (4) forces applied to a physics simulation that runs until computational equilibrium.  They are
1. A centering force that moves the center of mass of all nodes toward the center of the screen.
2. A collision force that prevents nodes from overlapping.
3. An ambient "charge" force that models each node as a negative point charge, encouraging nodes to move apart if there were no other forces applied.
4. A link force that acts differently depending on the strength of the similarity
   1. If the strength of the relationship is _greater than average_, a force similar to a spring is applied, `k * r`, where `k` is proportional to the strength of the relationship, and `r` is the distance between the two nodes.
   2. If the strength of the relationship is _less than average_, a force similar to an inverse spring is applied, `k / r`, where `k` is proportional to the strength of the relationship, and `r` is the distance between the two nodes.

There is also a hover tooltip that shows the user's top ten associations by strength of similarity.

### Sample Configuration

```toml
server_port = 8082
static_location = "../client/build/default"

with_analyzer = true

twitter_consumer_key = "..."
twitter_consumer_secret = "..."
twitter_access_token = "..."
twitter_access_secret = "..."

mongo_endpoint = "mongodb://localhost:27017"

signature_length = 1000
min_shingle_size = 3
max_shingle_size = 3
num_shingles_evaluated = 200

twitter_handles = [
    "BarackObama",
    "justinbieber",
    "katyperry",
    "rihanna",
    "taylorswift13",
    "Cristiano",
    "ladygaga",
    "realDonaldTrump",
]
```

### Build

```bash
docker build -t tweet-analyzer .
```

### Tests

Not yet.

## Conclusions

## License

```
The MIT License (MIT)

Copyright (c) 2020 Aaron Roney

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```