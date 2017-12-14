Test s3 functionality

  $ kitchensync() { RUST_BACKTRACE=1 $TESTDIR/../target/debug/kitchensync $* ; }

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

Now push to s3

  $ aws s3 rm --quiet --recursive s3://kitchensync-test/cram/b/

  $ kitchensync sync --push s3://kitchensync-test/cram/b/
  syncing to s3://kitchensync-test/cram/b/
  A hello
  A world
  sync successful

  $ aws s3 cp s3://kitchensync-test/cram/b/hello -
  hello

Make a change in a and update

  $ echo "hello world" > hello
  $ touch -d 1 hello

  $ kitchensync update
  updating
  U hello
  update successful

Dry run sync

  $ kitchensync sync --push --dry-run s3://kitchensync-test/cram/b/
  syncing to s3://kitchensync-test/cram/b/
  U hello
  sync successful

Real sync

  $ kitchensync sync --push s3://kitchensync-test/cram/b/
  syncing to s3://kitchensync-test/cram/b/
  U hello
  sync successful

  $ aws s3 cp s3://kitchensync-test/cram/b/hello -
  hello world

  $ aws s3 ls s3://kitchensync-test/cram/b/
  \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}        116 .kitchensync (re)
  \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}         12 hello (re)
  \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}          6 world (re)


Test deleting files

  $ rm world
  $ kitchensync update --deleted
  updating
  D world
  update successful

  $ kitchensync sync --delete --push s3://kitchensync-test/cram/b/
  syncing to s3://kitchensync-test/cram/b/
  R world
  sync successful

  $ aws s3 ls s3://kitchensync-test/cram/b/world
  [1]
