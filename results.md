benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt (2020 mb)
Ours: 55.11 seconds, 1876 mb
Theirs: 675 seconds, 6786 mb

benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt (2060 mb)
Ours: 46.66 seconds, 1939 mb
Theirs: 645 seconds, 5644 mb

benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt (2460 mb)
Ours: 86.48 seconds, 3798 mb
Theirs: 1377 seconds, 7029 mb

benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt (99 mb)
Ours: 4.50 seconds, 143 mb
Theirs: 2960 seconds, 379 mb

benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt (874 mb)
Ours: 14.78 seconds, 1016 mb
Theirs: 406 seconds, 1690 mb



# naive
Starting parsing benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt...
Parsing done, size: 284219250 in 15.259033 seconds
Number of states: 944250, Number of partitions: 944250
Computation took 14.684058 seconds
27.42user 2.56system 0:30.08elapsed 99%CPU (0avgtext+0avgdata 1188068maxresident)k
0inputs+0outputs (0major+332789minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt...
Parsing done, size: 303404990 in 17.203121 seconds
Number of states: 1007990, Number of partitions: 1007990
Computation took 16.89829 seconds
30.78user 3.36system 0:34.17elapsed 99%CPU (0avgtext+0avgdata 1263940maxresident)k
0inputs+0outputs (0major+605227minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt...
Parsing done, size: 261300000 in 21.6536 seconds
Number of states: 1300000, Number of partitions: 1300000
Computation took 29.344816 seconds
46.98user 3.99system 0:51.06elapsed 99%CPU (0avgtext+0avgdata 1110184maxresident)k
0inputs+0outputs (0major+344016minor)pagefaults 0swaps

Starting parsing benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt...
Parsing done, size: 13753169 in 1.415956 seconds
Number of states: 1632799, Number of partitions: 357456
Computation took 105.070885 seconds
104.64user 1.81system 1:46.49elapsed 99%CPU (0avgtext+0avgdata 111916maxresident)k
0inputs+0outputs (0major+1488176minor)pagefaults 0swaps

Starting parsing benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt...
Parsing done, size: 120061359 in 4.911465 seconds
Number of states: 4459455, Number of partitions: 4459455
Computation took 25.283943 seconds
29.07user 1.14system 0:30.23elapsed 99%CPU (0avgtext+0avgdata 762820maxresident)k
0inputs+0outputs (0major+812773minor)pagefaults 0swaps


# n log n
Starting parsing benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt...
Parsing done, size: 284219250 in 15.500636 seconds
Number of iterations: 3023
Number of states: 944250, Number of partitions: 944250
Computation took 17.755272 seconds
30.54user 2.69system 0:33.44elapsed 99%CPU (0avgtext+0avgdata 1718476maxresident)k
0inputs+0outputs (0major+449083minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt...
Parsing done, size: 303404990 in 17.207455 seconds
Number of iterations: 166029
Number of states: 1007990, Number of partitions: 1007990
Computation took 18.972242 seconds
32.94user 3.21system 0:36.19elapsed 99%CPU (0avgtext+0avgdata 1847772maxresident)k
0inputs+0outputs (0major+488083minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt...
Parsing done, size: 261300000 in 20.759348 seconds
Number of iterations: 47732
Number of states: 1300000, Number of partitions: 1300000
Computation took 26.221869 seconds
43.03user 3.94system 0:46.99elapsed 99%CPU (0avgtext+0avgdata 1876208maxresident)k
0inputs+0outputs (0major+505895minor)pagefaults 0swaps

Starting parsing benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt...
Parsing done, size: 13753169 in 1.343467 seconds
Number of iterations: 73780
Number of states: 1632799, Number of partitions: 357456
Computation took 2.42273 seconds
3.70user 0.07system 0:03.79elapsed 99%CPU (0avgtext+0avgdata 135280maxresident)k
0inputs+0outputs (0major+42296minor)pagefaults 0swaps

Starting parsing benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt...
Parsing done, size: 120061359 in 4.904483 seconds
Number of iterations: 199273
Number of states: 4459455, Number of partitions: 4459455
Computation took 7.593619 seconds
11.95user 0.53system 0:12.50elapsed 99%CPU (0avgtext+0avgdata 791340maxresident)k
0inputs+0outputs (0major+362465minor)pagefaults 0swaps


# n log n (old)
Starting parsing benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt...
Parsing done, size: 284219250 in 15.545295 seconds
Number of iterations: 3023
Number of states: 944250, Number of partitions: 944250
Computation took 16.808077 seconds
29.01user 3.32system 0:32.51elapsed 99%CPU (0avgtext+0avgdata 1717932maxresident)k
0inputs+0outputs (0major+707833minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt...
Parsing done, size: 303404990 in 15.643317 seconds
Number of iterations: 166029
Number of states: 1007990, Number of partitions: 1007990
Computation took 18.78018 seconds
31.58user 2.82system 0:34.44elapsed 99%CPU (0avgtext+0avgdata 1851332maxresident)k
0inputs+0outputs (0major+484736minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt...
Parsing done, size: 261300000 in 20.076384 seconds
Number of iterations: 47732
Number of states: 1300000, Number of partitions: 1300000
Computation took 28.092323 seconds
44.45user 3.66system 0:48.19elapsed 99%CPU (0avgtext+0avgdata 1870852maxresident)k
0inputs+0outputs (0major+501072minor)pagefaults 0swaps

Starting parsing benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt...
Parsing done, size: 13753169 in 1.3944781 seconds
Number of iterations: 73780
Number of states: 1632799, Number of partitions: 357456
Computation took 2.495454 seconds
3.81user 0.07system 0:03.89elapsed 99%CPU (0avgtext+0avgdata 135364maxresident)k
0inputs+0outputs (0major+43809minor)pagefaults 0swaps

Starting parsing benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt...
Parsing done, size: 120061359 in 4.871518 seconds
Number of iterations: 199273
Number of states: 4459455, Number of partitions: 4459455
Computation took 8.513905 seconds
12.86user 0.51system 0:13.39elapsed 99%CPU (0avgtext+0avgdata 793764maxresident)k
0inputs+0outputs (0major+299704minor)pagefaults 0swaps


# n log n (new)
Starting parsing benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt...
Parsing done, size: 284219250 in 16.68743 seconds
Number of iterations: 3023
Number of states: 944250, Number of partitions: 944250
Computation took 16.750526 seconds
30.44user 2.97system 0:33.61elapsed 99%CPU (0avgtext+0avgdata 1717936maxresident)k
0inputs+0outputs (0major+462075minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt...
Parsing done, size: 303404990 in 17.091637 seconds
Number of iterations: 166029
Number of states: 1007990, Number of partitions: 1007990
Computation took 24.255669 seconds
37.85user 3.46system 0:41.35elapsed 99%CPU (0avgtext+0avgdata 1851364maxresident)k
0inputs+0outputs (0major+744834minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt...
Parsing done, size: 261300000 in 20.978249 seconds
Number of iterations: 47732
Number of states: 1300000, Number of partitions: 1300000
Computation took 27.495255 seconds
44.86user 3.56system 0:48.49elapsed 99%CPU (0avgtext+0avgdata 1870896maxresident)k
0inputs+0outputs (0major+515420minor)pagefaults 0swaps

Starting parsing benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt...
Parsing done, size: 13753169 in 1.415843 seconds
Number of iterations: 73780
Number of states: 1632799, Number of partitions: 357456
Computation took 2.431604 seconds
3.77user 0.07system 0:03.85elapsed 99%CPU (0avgtext+0avgdata 135372maxresident)k
0inputs+0outputs (0major+43849minor)pagefaults 0swaps

Starting parsing benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt...
Parsing done, size: 120061359 in 5.57091 seconds
Number of iterations: 199273
Number of states: 4459455, Number of partitions: 4459455
Computation took 7.999104 seconds
12.96user 0.59system 0:13.59elapsed 99%CPU (0avgtext+0avgdata 794740maxresident)k
0inputs+0outputs (0major+359041minor)pagefaults 0swaps