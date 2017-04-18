# FRC Elo

This program is an Elo ranking system for FIRST Robotics.

## Purpose

Elo ratings are designed as an estimate of a player's skill. This
project is mostly for fun. It is interesting to see how a team's
competitive strength has changed over time. Elo ratings are also
predictive to some extent, so this system could potentially be
used to estimate the probabilities of future outcomes.

## Introduction

I have been interested in applying statistical models to FRC for
several years. This program has its roots in a python program I wrote
in 2014. This was not the first nor the best elo algorithm and many
other elo systems can be found
on
[Chief Delphi](https://www.chiefdelphi.com/forums/search.php?searchid=8141035)

The ranking system used here is based on the system described
by Caleb Sykes of [Team 4536](http://www.minutebots.org/).

## Installation

This program is build
using [Diesel](http://diesel.rs/), [SQLite](https://www.sqlite.org/),
and [Rust](https://www.rust-lang.org/en-US/). To install,
first [install Rust](https://www.rust-lang.org/en-US/install.html) and
then install `diesel_cli` by running

```
cargo install diesel_cli
```

For this install to work, you will need to have an implementation of
both PostgreSQL and MySQL installed.

Clone the git repository.

```
git clone git@github.com:CarlColglazier/frc-elo.git && cd frc-elo
```

Set up the environment by writing the following into `.env`:

```
DATABASE_URL=<database-file>
TBA_KEY=<key>
```

`TBA_KEY` can be
generated [here](https://www.thebluealliance.com/account).

Now run `diesel setup && diesel migration run`. This sets up the
database.

We can now build the executable by running `cargo build
--release`. This should take several minutes as cargo has to fetch,
compile, and optimize all the dependencies.

Use the newly compiled program to fetch the historic data from The
Blue Alliance. Run `./target/release/frc-elo sync`. While the program
takes advantage of multithreaded programming, this process will still
take about three minutes. Making the requests to The Blue Alliance is
rather fast, but committing everything to the database is currently a
bit of a bottleneck. This would be a good place for a future
improvement.

The program keeps track of when each request to The Blue Alliance was
last updated, so future `sync` requests should take no more than a
minute.
