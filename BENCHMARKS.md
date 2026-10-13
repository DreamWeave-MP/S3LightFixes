# S3LightFixes benchmarks

`bench/bench.luau` times the tool the way a caller reaches it: `@s3lightfixes` over an `@openmw`
Context, and `@s3lightfixes/cli`, for an install the bench writes. The install is a master of
1,200 lights and 1,500 cells (and 300 statics the tool is told to leave out) and 24 plugins of 60
lights and 40 cells each, a third of which override an earlier record under an id in another
case: 2,640 light records of which 2,160 are fixed, and 2,460 cells. It runs the lights alone
(the default), the lights and every interior cell, a dry run, a run under a lightconfig.toml of 17
exclusions, 3 sun cells, 25 light overrides and 13 cell overrides that is saved again, a
validation of that configuration, the reading of a command line of 46 arguments, and the whole
command. Each workload is warmed up, sized to a batch of about a tenth of a second, and reported
as the median of seven batches, with a full collection between them. Every run's report, plugin,
log, and saved configuration are held to the SHA-256 of what the tool wrote before anything here
was changed. The tool prints its log, as it does from the command line, so the rows are the lines
with a tab in them.

`bench/cold.luau` times what one process pays once: the `require` of this package's two modules
with the modules below them loaded first, then the first run of the command over a small install
(72 lights fixed), and a second with its 20 cells. Each line is a single sample, so it is run many
times and the medians are compared.

```sh
jess run --package jpk/s3lightfixes --package-path jpk \
  --capability filesystem.read --capability filesystem.write --capability process.environment \
  jpk/s3lightfixes/bench/bench.luau <scratch folder> [filter] | grep -P '\t'

jess run --package jpk/s3lightfixes --package-path jpk \
  --capability filesystem.read --capability filesystem.write --capability process.environment \
  jpk/s3lightfixes/bench/cold.luau <scratch folder> | grep -P '\t'
```

## The move onto l3i 2.0, base against head

- Machine: Intel Core i7-10870H @ 2.20 GHz, 16 threads, shared with other work while this ran
  (load average 1.0 to 1.9 over the runs).
- Date: 2026-10-13.
- Binary: the verified release build of Jess 0.1.0 on l3i 2.0.2 (Luau 0.742), the same one for
  both sides, and the same sibling packages (`tes3` at `d1c77ed`, `load-order` at `d07e699`,
  `openmw` at `515f5b5`).
- Base: `2e36314`, the package before the move, run with the benches as they are now. Head:
  `31bc4bf`.
- Protocol: the whole of `bench.luau` against an export of base, then head, five times over,
  back to back under one lock; each cell is the median of its five runs. `cold.luau` the same
  way, thirty-one times over. A difference under 5 percent is called no change.
- The table is the first of three sessions. The second, the same protocol a minute later, had
  1.32x, 1.20x, 1.38x, 1.43x, no change, no change and 1.34x down the warm rows, and 0.97x,
  1.08x, 1.06x and no change down the cold ones from the second. The third is the
  change-at-a-time run below, whose first and last columns are base and head: 1.33x, 1.25x,
  1.33x, 1.44x, no change, no change, 1.38x, and cold 0.98x, 1.10x, 1.09x, no change.

| Workload | Base, µs | Head, µs | Base / head |
|---|---:|---:|---:|
| run: the lights of 25 plugins | 55892.6 | 42447.2 | 1.32x |
| run: the lights and cells, no sun in any interior | 84401.8 | 68885.7 | 1.23x |
| run: a dry run of the lights | 30587.8 | 21060.7 | 1.45x |
| run: a lightconfig.toml of 24 light and 12 cell overrides, saved | 91868.1 | 62260.4 | 1.48x |
| validate: a lightconfig.toml of 24 light and 12 cell overrides | 4697.8 | 4618.8 | no change |
| cli: parse 46 arguments, 15 overrides among them | 193.5 | 189.3 | no change |
| cli: run the lights of 25 plugins | 51526.4 | 37737.3 | 1.37x |

| One cold process | Base, µs | Head, µs | Base / head |
|---|---:|---:|---:|
| require the modules below this package | 271178.4 | 272478.1 | no change |
| require @s3lightfixes/cli, those loaded | 25936.6 | 26797.7 | no change (0.97x) |
| first run, the lights of 3 plugins | 5729.9 | 5019.4 | 1.14x |
| second run, lights and cells | 4925.9 | 4539.4 | 1.09x |
| require @s3lightfixes/cli to the end of the first run | 31712.0 | 31926.1 | no change |

### What moved, and why

None of the gain is the dialect's. The tool keeps no numbers in buffers and sorts no numbers, so
the data plane has nothing of its to take; what it does for a light is a few dozen f32 operations
and then a great deal of text. The time went where the text was made:

- **The sort.** Lights are written sorted by id, by bytes, and were sorted by a comparator that
  walked two ids a byte at a time in Luau, with a table of places to break ties that can't
  happen (a light is taken only when its id, lowered, is new). Luau orders strings by bytes
  itself, so the ids are sorted as strings with no comparator and the lights taken in that
  order: 5.9 ms to 0.7 ms for 2,160 lights.
- **The flags.** A light's flags come from `@tes3` as text (`DYNAMIC | FIRE`), were split,
  trimmed and looked up for every light, and spelled back for every light. A load order has a
  handful of sets of flags, so each is read and spelled once a run.
- **The log's text.** A change of color was two `table.concat`s inside an interpolation, a
  change of radius two `string.format`s inside another, and a log line two `string.gsub`s with a
  callback to escape what almost no id has. Each is now one concatenation or one format, a
  plugin's name is quoted once for all its lines, and an id is asked whether it has anything to
  escape by one call of a pattern before the `gsub` runs.
- **The patterns.** Every light and cell was tried against each exclusion in turn, each sun
  cell, and each override. Where a list has two patterns or more, one pattern of them all (each
  in a group of its own) says in one call whether any matches: instead of the loop for
  exclusions and sun cells, and before it for overrides, most of which most ids don't match. 17
  exclusions took 1,273 ns an id one by one and take 80 ns as one pattern. Making it costs about
  30 µs a pattern, which is why it is made after a validation has returned and never for the
  exclusions of plugins, tried once each. A pattern list with a `#` in it is tried one by one as
  before: where a pattern allows comments, a `#` starts one that would swallow the group's end.

What didn't move is where the rest of the time is. At head a run of the lights spends 18.4 of
its 32 ms in `@tes3` (decoding 2,640 records 5.8, encoding and inserting 2,160 9.7, opening 1.4,
saving 1.5) and 2.1 printing and writing the log; a validation is the compiling of 57 patterns;
the command line is read in 0.17 ms either way.

The cold rows say what a small install gets: the modules take 0.4 to 0.9 ms longer to `require`
(under what the protocol can show on that row, but the same sign in all three sessions), the
first run over 72 lights is 0.5 ms shorter, and the two together don't move. `--!native` has no
part in it: Jess compiles every module natively whatever the file says, and the two modules load
and run in the same time without the directive. Of the 24 ms, 21 are `src/init.luau`. Next to
them, the modules below this package take 250 ms.

### A change at a time

Each commit that touches the payload against the one before it: `bench.luau` five times and
`cold.luau` forty-five times over the eleven sources in turn, under one lock a round (warm) or
fifteen rounds (cold), medians, in µs. Ratios are before / after; a blank is no change. The cold
column's three ratios are under 5 percent, and are given because the next section is about them.

| Commit | What it does | lights | lights and cells | dry run | saved config | cold `require` |
|---|---|---:|---:|---:|---:|---:|
| `2e36314` | base | 50006.6 | 75696.2 | 26208.6 | 81506.8 | 24309.1 |
| `c6038f4` | ids sorted as strings | 45455.9 (1.10x) | 72931.1 | 26701.6 | 76725.3 (1.06x) | 24359.9 |
| `2cd6b03` | flags read and spelled once | 41630.5 (1.09x) | 67264.3 (1.08x) | 22730.6 (1.17x) | 72494.9 (1.06x) | 24338.9 |
| `1ec8f51` | a change as one string | 39394.5 (1.06x) | 63343.2 (1.06x) | 20744.4 (1.10x) | 70613.4 | 24419.2 |
| `9a709b9` | multipliers by name | 39858.6 | 64301.5 | 20552.0 | 69677.1 | 24553.9 |
| `7da16f7` | a log line as one string | 37881.4 (1.05x) | 61231.2 (1.05x) | 19201.1 (1.07x) | 68751.8 | 24753.4 |
| `158a725` | one pattern of them all | 37159.2 | 60587.5 | 18842.9 | 56091.5 (1.23x) | 25047.5 |
| `1c5d218` | four comprehensions | 37830.7 | 59484.2 | 19225.3 | 56424.2 | 25886.9 (0.97x) |
| `d03524f` | six record patterns | 37248.4 | 61033.9 | 18882.3 | 56270.6 | 26048.8 |
| `8511d26` | the four as loops again | 37536.3 | 61730.7 | 19207.1 | 56532.0 | 25548.1 (1.02x) |
| `31bc4bf` | three more as loops again | 37595.3 | 60317.3 | 19768.3 | 56530.1 | 24703.3 (1.03x) |

### The dialect, and what it costs to load

The payload has six dialect constructs, all record patterns: a color's channels (twice), an
HSV's three parts, the configuration's two tables of settings, a loaded plugin's three parts, and
the two configurations a command can name. They are the reads they replace. The suite and the
bench build their lists by comprehension.

It had seven comprehensions and reducers as well, each the exact loop it replaced: a flag set's
names, a TOML array's lines, a table's header, the tables to write, the ids to sort, a count of
the changed cells (`#[...]`), and whether any pattern matches (`any[...]`). None moved a warm row,
and together they cost a cold run 5 to 6 percent of the `require`, so they are loops again. The
same payload three ways, twenty-five runs each under one lock:

| Payload | `require`, µs | |
|---|---:|---|
| no dialect construct | 24661.6 | |
| the six record patterns | 24825.6 | no change |
| those and the seven comprehensions and reducers | 26221.3 | 0.95x |

Three more sessions gave the seven 1.3 to 1.6 ms: 25046.5 against 26333.3; 24514.5 against
26076.4, with the four outside the run 25216.8 and the three inside it 25353.9; and the table
above. That is about 0.2 ms a construct in this module of 64 KB. In a module of 150 small
functions the same constructs cost 44 µs (a comprehension), 36 µs (a count) and 41 µs (an `any`)
more than their loops, and a module's first about 0.2 ms more than that; a record pattern costs
4.5 µs. Added one at a time to the payload of loops, no construct could be told from none (within
0.35 ms either way, twenty-five runs), so the two commits that put them back are each under what
one row shows alone, and 1.05x together.

### Measured and left alone

- **A comprehension in the command line.** `fieldsIn` in `src/cli.luau` is a filter of
  `string.split`'s list, and exact as one. It would be that module's first construct: the
  reading of 46 arguments didn't move (172.4 µs against 171.7), and the `require` was 0.25 ms
  longer over forty-five runs (26204.1 against 26457.2), no change over twenty-one. Not taken.
- **A slice for a color's four bytes.** `list[1:4]` in place of `{ list[1], list[2], list[3],
  list[4] }` is 116 ns against 59. Not taken.
- **Every id of a plugin tried in one call.** `@dream/bytes`' `matchSpans` over 2,000 ids and
  the one pattern of 17 is 56 µs, against 149 µs for one `isMatch` an id (and 2,500 µs for 17
  each). It needs every id before any light is fixed, so a pass to decode a plugin's records
  and another to fix them, and with that a record that doesn't decode would stop the run before
  an earlier record's own error did. A tenth of a millisecond in 2,000 ids. Not taken.
- **The one pattern only for long runs.** It costs about 30 µs a pattern and saves about 75 ns
  an id for each pattern after the first, so it pays for itself at 400 to 800 ids: any real
  install, and not one of 72 lights, which by that arithmetic loses about 0.1 ms to three
  patterns. Counting ids before making it would save that and no more. Not taken.
- **Reading an id before decoding a record.** A light or cell whose id is taken is decoded only
  to learn that (480 of the bench's 2,640 lights). `tes3.records.id` reads it without decoding,
  but a record that doesn't decode and whose id is taken would then be passed over where it
  stops the run today. Not taken.
