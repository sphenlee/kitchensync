Test duplicating files

  $ kitchensync() { RAYON_NUM_THREADS=1 RUST_BACKTRACE=1 $TESTDIR/../target/debug/kitchensync $* ; }

  $ mkdir a
  $ cd a

  $ echo hello > hello
  $ touch -d 0 hello

  $ kitchensync update
  updating
  A hello
  update successful

Now sync to a new directory

  $ mkdir ../b
  $ cd ../b

  $ kitchensync sync ../a
  syncing from ../a
  A hello
  sync successful

  $ cat hello
  hello

Move a file and sync again

  $ cd ../a
  $ mv hello world

  $ kitchensync -v update --deleted
  updating
  getting files
  looking for changes
  checking "world"
  A world
  reporting deleted files
  D hello
  update successful

  $ cd ../b
  $ kitchensync sync --delete ../a
  syncing from ../a
  A world
  R hello
  sync successful

  $ cat world
  hello

  $ test -f hello
  [1]
