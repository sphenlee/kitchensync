Test push functionality of the sync command

  $ kitchensync() { RAYON_NUM_THREADS=1 $TESTDIR/../target/debug/kitchensync $* ; }

  $ mkdir a
  $ cd a

  $ echo hello > hello
  $ echo world > world
  $ touch -d 0 hello
  $ touch -d 0 world

  $ kitchensync update
  updating
  A world
  A hello
  update successful

Now push to a new directory

  $ mkdir ../b

  $ kitchensync sync --push ../b
  syncing to ../b
  A world
  A hello
  sync successful

  $ cat hello
  hello

Make a change in a and update

  $ echo "hello world" > hello
  $ touch -d 1 hello

  $ kitchensync update
  updating
  U hello
  update successful

Dry run sync

  $ kitchensync sync --push --dry-run ../b
  syncing to ../b
  U hello
  sync successful

Real sync

  $ kitchensync sync --push ../b
  syncing to ../b
  U hello
  sync successful

  $ cat ../b/hello
  hello world

  $ ls ../b
  hello
  world

Test deleting files

  $ rm world
  $ kitchensync update --deleted
  updating
  D world
  update successful

Create the deleted file again to check we delete the right
file in the sync below

  $ echo tombstone > world

  $ kitchensync sync --delete --push ../b
  syncing to ../b
  R world
  sync successful

  $ cat world
  tombstone

  $ test -f ../b/world
  [1]
