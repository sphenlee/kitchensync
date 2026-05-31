Test basic functionality of the sync command

  $ kitchensync() { RAYON_NUM_THREADS=1 RUST_BACKTRACE=1 $TESTDIR/../target/debug/kitchensync $* ; }

  $ mkdir a
  $ cd a

  $ echo hello > hello
  $ echo world > world
  $ touch -d 0 hello
  $ touch -d 0 world

  $ kitchensync update
  updating
  A hello
  A world
  update successful

Now sync to a new directory

  $ mkdir ../b
  $ cd ../b

  $ kitchensync sync ../a
  syncing from ../a
  A hello
  A world
  sync successful

  $ ls
  hello
  world

  $ cat hello
  hello

Make a change in a and update

  $ cd ../a
  $ echo "hello world" > hello
  $ touch -d 1 hello

  $ kitchensync update
  updating
  U hello
  update successful

Dry run sync

  $ cd ../b
  $ kitchensync sync --dry-run ../a
  syncing from ../a
  U hello
  sync successful

Real sync

  $ kitchensync sync ../a
  syncing from ../a
  U hello
  sync successful

  $ cat hello
  hello world

Test deleting files

  $ cd ../a
  $ rm world
  $ kitchensync update --deleted
  updating
  D world
  update successful

  $ cd ../b
  $ kitchensync sync ../a
  syncing from ../a
  sync successful

  $ cat world
  world

  $ kitchensync update
  updating
  A world
  update successful

  $ kitchensync sync --dry-run --delete ../a
  syncing from ../a
  R world
  sync successful

  $ cat world
  world

  $ kitchensync sync --delete ../a
  syncing from ../a
  R world
  sync successful

  $ test -f world
  [1]
