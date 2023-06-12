# TSC Simulator

A tool to simulate guest TSCs based on their host TSC, across migrations.
Also provides utilities to perform time-related calculations that come up when
doing work with the TSC.

## Modes

This tool has two subcommands: `calc` and `simulate`.

The `calc` command is for
making common calculations associated with high-resolution time, such as
converting between hrtime and a TSC reading, computing a guest TSC based on
input parameters, or finding the correct a guest TSC offset. The `calc`
subcommands offers an `implementation` parameter, which implements some of the
core math functions, which require 128-bit intermediate representations, in
either Rust or assembly.

The `simulate` command is to simulate the value of a guest TSC over time,
including following live migration(s).

Each of the subcommands and sub-subcommands offer a variety of parameters
related to making the calculations, including the integer and fractional bit
size of the fixed-point number used to represent the guest/host frequency ratio.

See the `help` subcommands for details.


### `calc` examples

#### Virtualized Guest TSC

Find the current guest TSC for a guest that booted when its host TSC was
300000000000, for a current host TSC of 305000000000:

```
$ tsc-simulator calc guest-tsc -i 300000000000 305000000000

calculating guest TSC for parameters:
	Host:
		initial TSC: 300000000000 (0x45d964b800)
		current TSC: 305000000000 (0x47036aaa00)
		frequency: 1000000000 Hz
	Guest:
		initial TSC: 0 (0x0)
		frequency: 1000000000 Hz
	Implementation: Rust

Guest TSC: 5000000000 (0x12a05f200)
```

### `simulate` examples

Simulate a guest running for 20 seconds, with a frequency of 1GHz, on a host
with frequency 1GHz, in which the guest booted after the host was up for 1
second:

```
$ tsc-simulator simulate

 DURATION        20 seconds                       
 GUEST FREQUENCY 1000000000 Hz                            

 HOST 0         
      START TIME 0 seconds                       
             TSC 1000000000                    
       FREQUENCY 1000000000 Hz                            


TIME              GUEST_TSC         HOST_TSC
=== GUEST_BOOT ==================================================================
0                         0       1000000000
1                1000000000       2000000000
2                2000000000       3000000000
3                3000000000       4000000000
4                4000000000       5000000000
5                5000000000       6000000000
6                6000000000       7000000000
7                7000000000       8000000000
8                8000000000       9000000000
9                9000000000      10000000000
10              10000000000      11000000000
11              11000000000      12000000000
12              12000000000      13000000000
13              13000000000      14000000000
14              14000000000      15000000000
15              15000000000      16000000000
16              16000000000      17000000000
17              17000000000      18000000000
18              18000000000      19000000000
19              19000000000      20000000000
20              20000000000      21000000000
```

A guest with 0.5GHz running on a host of 2GHz:

```
tsc-simulator simulate -g 500000000 -f 2000000000

 DURATION        20 seconds                       
 GUEST FREQUENCY 500000000 Hz                            

 HOST 0         
      START TIME 0 seconds                       
             TSC 1000000000                    
       FREQUENCY 2000000000 Hz                            


TIME              GUEST_TSC         HOST_TSC
=== GUEST_BOOT ==================================================================
0                         0       1000000000
1                 500000000       3000000000
2                1000000000       5000000000
3                1500000000       7000000000
4                2000000000       9000000000
5                2500000000      11000000000
6                3000000000      13000000000
7                3500000000      15000000000
8                4000000000      17000000000
9                4500000000      19000000000
10               5000000000      21000000000
11               5500000000      23000000000
12               6000000000      25000000000
13               6500000000      27000000000
14               7000000000      29000000000
15               7500000000      31000000000
16               8000000000      33000000000
17               8500000000      35000000000
18               9000000000      37000000000
19               9500000000      39000000000
20              10000000000      41000000000
```

A guest that is migrated after 10 seconds from a host with 1GHz, to a host with
2GHz:
```
tsc-simulator simulate --migrate "10 10000000000 2000000000"

 DURATION        20 seconds                       
 GUEST FREQUENCY 1000000000 Hz                            

 HOST 0         
      START TIME 0 seconds                       
             TSC 1000000000                    
       FREQUENCY 1000000000 Hz                            

 HOST 1         
      START TIME 10 seconds                       
             TSC 10000000000                   
       FREQUENCY 2000000000 Hz                            


TIME              GUEST_TSC         HOST_TSC
=== GUEST_BOOT ==================================================================
0                         0       1000000000
1                1000000000       2000000000
2                2000000000       3000000000
3                3000000000       4000000000
4                4000000000       5000000000
5                5000000000       6000000000
6                6000000000       7000000000
7                7000000000       8000000000
8                8000000000       9000000000
9                9000000000      10000000000
10              10000000000      11000000000
=== MIGRATION 1 =================================================================
10              10000000000      10000000000
11              11000000000      12000000000
12              12000000000      14000000000
13              13000000000      16000000000
14              14000000000      18000000000
15              15000000000      20000000000
16              16000000000      22000000000
17              17000000000      24000000000
18              18000000000      26000000000
19              19000000000      28000000000
20              20000000000      30000000000
```

Multiple migrations:
```
simulate -d 15 --migrate "5 300000000000 2000000000" --migrate "10 100000000000 1500000000"

 DURATION        15 seconds                       
 GUEST FREQUENCY 1000000000 Hz                            

 HOST 0         
      START TIME 0 seconds                       
             TSC 1000000000                    
       FREQUENCY 1000000000 Hz                            

 HOST 1         
      START TIME 5 seconds                       
             TSC 300000000000                  
       FREQUENCY 2000000000 Hz                            

 HOST 2         
      START TIME 10 seconds                       
             TSC 100000000000                  
       FREQUENCY 1500000000 Hz                            


TIME              GUEST_TSC         HOST_TSC
=== GUEST_BOOT ==================================================================
0                         0       1000000000
1                1000000000       2000000000
2                2000000000       3000000000
3                3000000000       4000000000
4                4000000000       5000000000
5                5000000000       6000000000
=== MIGRATION 1 =================================================================
5                5000000000     300000000000
6                6000000000     302000000000
7                7000000000     304000000000
8                8000000000     306000000000
9                9000000000     308000000000
10              10000000000     310000000000
=== MIGRATION 2 =================================================================
10              10000000000     100000000000
11              10999999999     101500000000
12              11999999999     103000000000
13              12999999999     104500000000
14              13999999999     106000000000
15              14999999998     107500000000
```
