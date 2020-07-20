Tests basic functionality of the update command

  $ kitchensync() { $TESTDIR/../target/debug/kitchensync $* ; }

  $ mkdir a
  $ cd a

  $ echo hello > hello
  $ echo world > world
  $ touch -d 0 hello
  $ touch -d 0 world

Initial update

  $ kitchensync update
  updating
  A hello
  A world
  update successful

Touch a file

  $ echo "hello world" > hello
  $ touch -d 1 hello

  $ kitchensync update
  updating
  U hello
  update successful

No changes

  $ kitchensync update
  updating
  update successful

Check the store

  $ cat .kitchensync
  22596363b3de40b06f981fb85d82312e8c0ed511 1595206800 hello
  9591818c07e900db7e1e0bc4b884c945e6a61b24 1595203200 world
