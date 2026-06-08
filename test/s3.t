Test s3 functionality

  $ kitchensync() { RAYON_NUM_THREADS=1 $TESTDIR/../target/debug/kitchensync $* ; }

  $ mkdir a
  $ cd a

  $ echo hello > hello
  $ echo world > world
  $ touch -d 2020-01-01T00:00:00 hello
  $ touch -d 2020-01-01T00:00:00 world

  $ kitchensync update
  updating
  A hello
  A world
  update successful

Configure AWS

  $ export AWS_DEFAULT_REGION=us-east-1

  $ aws sts get-caller-identity
  {
      "UserId": "[A-Z0-9]+",  (re)
      "Account": "120228355863",
      "Arn": "arn:aws:iam::120228355863:.*" (re)
  }

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
  $ touch -d 2020-01-01T00:00:01 hello

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

Test pulling

  $ cd ..
  $ mkdir b
  $ cd b
  $ kitchensync sync s3://kitchensync-test/cram/b/
  syncing from s3://kitchensync-test/cram/b/
  A hello
  sync successful
  $ cat .kitchensync
  22596363b3de40b06f981fb85d82312e8c0ed511 1577836801 hello
  $ cat hello
  hello world
