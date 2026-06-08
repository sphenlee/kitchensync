Test basic functionality of the sync command

  $ kitchensync() { $TESTDIR/../target/debug/kitchensync $* ; }

  $ mkdir a
  $ cd a

  $ echo hello > hello
  $ touch -d 0 hello

  $ kitchensync update
  updating
  A hello
  update successful
s
Now sync to a new directory

  $ mkdir ../b
  $ cd ../b

Create the config file

  $ cat > .kitchensync.toml <<EOF
  > [[destination]]
  > name = "default"
  > target = "../a"
  > EOF

  $ kitchensync update
  updating
  update successful

  $ kitchensync sync
  syncing from ../a
  A hello
  sync successful

  $ ls
  hello
