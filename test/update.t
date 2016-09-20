Tests basic functionality of the update command

  $ kitchensync() { $TESTDIR/../target/debug/kitchensync $* ; }

  $ mkdir a
  $ cd a

  $ echo hello > hello
  $ echo world > world

Initial update

  $ kitchensync update
  # updating
  A ./hello
  A ./world
  # update successful

Touch a file

  $ sleep 1
  $ echo "hello world" > hello
  $ kitchensync update
  # updating
  U ./hello
  # update successful

No changes

  $ kitchensync update
  # updating
  # update successful
