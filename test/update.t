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
  # updating
  A ./hello
  A ./world
  # update successful

Touch a file

  $ echo "hello world" > hello
  $ touch -d 1 hello

  $ kitchensync update
  # updating
  U ./hello
  # update successful

No changes

  $ kitchensync update
  # updating
  # update successful
