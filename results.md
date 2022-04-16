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


# FxHash
Starting parsing benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt...
Parsing done, size: 284219250 in 3.5574899 seconds
Number of iterations: 1
Number of states: 944250, Number of partitions: 944250
Computation took 2.7414489 seconds
5.71user 0.65system 0:06.52elapsed 97%CPU (0avgtext+0avgdata 1206924maxresident)k
0inputs+0outputs (0major+301864minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt...
Parsing done, size: 303404990 in 4.397976 seconds
Number of iterations: 1
Number of states: 1007990, Number of partitions: 1007990
Computation took 2.935729 seconds
6.37user 1.07system 0:07.48elapsed 99%CPU (0avgtext+0avgdata 2265836maxresident)k
0inputs+0outputs (0major+566592minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt...
Parsing done, size: 261300000 in 4.467041 seconds
Number of iterations: 2
Number of states: 1300000, Number of partitions: 1300000
Computation took 5.447722 seconds
9.19user 0.75system 0:09.98elapsed 99%CPU (0avgtext+0avgdata 1159308maxresident)k
0inputs+0outputs (0major+289960minor)pagefaults 0swaps

Starting parsing benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt...
Parsing done, size: 13753169 in 0.19657299 seconds
Number of iterations: 186
Number of states: 1632799, Number of partitions: 357456
Computation took 27.89051 seconds
27.70user 0.36system 0:28.09elapsed 99%CPU (0avgtext+0avgdata 119536maxresident)k
0inputs+0outputs (0major+243924minor)pagefaults 0swaps

Starting parsing benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt...
Parsing done, size: 120061359 in 1.380167 seconds
Number of iterations: 7
Number of states: 4459455, Number of partitions: 4459455
Computation took 7.635571 seconds
8.33user 0.71system 0:09.07elapsed 99%CPU (0avgtext+0avgdata 898612maxresident)k
0inputs+0outputs (0major+369923minor)pagefaults 0swaps

# AHash
Starting parsing benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt...
Parsing done, size: 284219250 in 3.6182 seconds
Number of iterations: 1
Number of states: 944250, Number of partitions: 944250
Computation took 2.941385 seconds
5.96user 0.65system 0:06.78elapsed 97%CPU (0avgtext+0avgdata 1251692maxresident)k
0inputs+0outputs (0major+313056minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt...
Parsing done, size: 303404990 in 4.403243 seconds
Number of iterations: 1
Number of states: 1007990, Number of partitions: 1007990
Computation took 3.268627 seconds
6.74user 1.05system 0:07.82elapsed 99%CPU (0avgtext+0avgdata 2265956maxresident)k
0inputs+0outputs (0major+566622minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt...
Parsing done, size: 261300000 in 4.679935 seconds
Number of iterations: 2
Number of states: 1300000, Number of partitions: 1300000
Computation took 6.6870623 seconds
10.57user 0.82system 0:11.43elapsed 99%CPU (0avgtext+0avgdata 1156368maxresident)k
0inputs+0outputs (0major+289225minor)pagefaults 0swaps

Starting parsing benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt...
Parsing done, size: 13753169 in 0.21518701 seconds
Number of iterations: 186
Number of states: 1632799, Number of partitions: 357456
Computation took 29.580109 seconds
29.40user 0.36system 0:29.80elapsed 99%CPU (0avgtext+0avgdata 112148maxresident)k
0inputs+0outputs (0major+272227minor)pagefaults 0swaps

Starting parsing benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt...
Parsing done, size: 120061359 in 1.488159 seconds
Number of iterations: 7
Number of states: 4459455, Number of partitions: 4459455
Computation took 8.07678 seconds
8.90user 0.69system 0:09.62elapsed 99%CPU (0avgtext+0avgdata 1007416maxresident)k
0inputs+0outputs (0major+327128minor)pagefaults 0swaps


# n log n
Starting parsing benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt...
Parsing done, size: 284219250 in 3.46316 seconds
Number of iterations: 3023
Number of states: 944250, Number of partitions: 944250
Computation took 11.79222 seconds
14.51user 0.84system 0:15.58elapsed 98%CPU (0avgtext+0avgdata 1765812maxresident)k
0inputs+0outputs (0major+450212minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt...
Parsing done, size: 303404990 in 3.659169 seconds
Number of iterations: 166029
Number of states: 1007990, Number of partitions: 1007990
Computation took 13.524871 seconds
16.35user 0.91system 0:17.30elapsed 99%CPU (0avgtext+0avgdata 1876032maxresident)k
0inputs+0outputs (0major+477249minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt...
Parsing done, size: 261300000 in 4.197608 seconds
Number of iterations: 47732
Number of states: 1300000, Number of partitions: 1300000
Computation took 18.104982 seconds
21.36user 1.02system 0:22.42elapsed 99%CPU (0avgtext+0avgdata 1883520maxresident)k
0inputs+0outputs (0major+472866minor)pagefaults 0swaps

Starting parsing benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt...
Parsing done, size: 13753169 in 0.191296 seconds
Number of iterations: 73780
Number of states: 1632799, Number of partitions: 357456
Computation took 0.819024 seconds
0.95user 0.05system 0:01.01elapsed 99%CPU (0avgtext+0avgdata 126644maxresident)k
0inputs+0outputs (0major+31802minor)pagefaults 0swaps

Starting parsing benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt...
Parsing done, size: 120061359 in 1.276857 seconds
Number of iterations: 199273
Number of states: 4459455, Number of partitions: 4459455
Computation took 4.440344 seconds
5.34user 0.41system 0:05.77elapsed 99%CPU (0avgtext+0avgdata 873436maxresident)k
0inputs+0outputs (0major+227175minor)pagefaults 0swaps


# unsafe
Starting parsing benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt...
Parsing done, size: 284219250 in 3.63469 seconds
Number of iterations: 1
Number of states: 944250, Number of partitions: 944250
Computation took 2.610657 seconds
5.54user 0.74system 0:06.46elapsed 97%CPU (0avgtext+0avgdata 1252144maxresident)k
0inputs+0outputs (0major+313169minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt...
Parsing done, size: 303404990 in 3.8978019 seconds
Number of iterations: 1
Number of states: 1007990, Number of partitions: 1007990
Computation took 2.885907 seconds
6.00user 0.77system 0:06.86elapsed 98%CPU (0avgtext+0avgdata 1328232maxresident)k
0inputs+0outputs (0major+332191minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt...
Parsing done, size: 261300000 in 4.86959 seconds
Number of iterations: 2
Number of states: 1300000, Number of partitions: 1300000
Computation took 6.071471 seconds
10.03user 0.90system 0:11.00elapsed 99%CPU (0avgtext+0avgdata 1117448maxresident)k
0inputs+0outputs (0major+279495minor)pagefaults 0swaps

Starting parsing benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt...
Parsing done, size: 13753169 in 0.18938 seconds
Number of iterations: 186
Number of states: 1632799, Number of partitions: 357456
Computation took 25.236647 seconds
25.06user 0.36system 0:25.43elapsed 99%CPU (0avgtext+0avgdata 121340maxresident)k
0inputs+0outputs (0major+288459minor)pagefaults 0swaps

Starting parsing benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt...
Parsing done, size: 120061359 in 1.361123 seconds
Number of iterations: 7
Number of states: 4459455, Number of partitions: 4459455
Computation took 8.049303 seconds
8.71user 0.72system 0:09.47elapsed 99%CPU (0avgtext+0avgdata 1007436maxresident)k
0inputs+0outputs (0major+327100minor)pagefaults 0swaps

# safe
Starting parsing benchmarks/large/wta_Word,or_0,0,0,4_47212500_50_944250_32.boa.txt...
Parsing done, size: 284219250 in 3.808344 seconds
Number of iterations: 1
Number of states: 944250, Number of partitions: 944250
Computation took 2.813301 seconds
5.94user 0.73system 0:06.91elapsed 96%CPU (0avgtext+0avgdata 1251664maxresident)k
0inputs+0outputs (0major+313049minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt...
Parsing done, size: 303404990 in 3.920388 seconds
Number of iterations: 1
Number of states: 1007990, Number of partitions: 1007990
Computation took 2.8842869 seconds
6.12user 0.73system 0:06.88elapsed 99%CPU (0avgtext+0avgdata 1290308maxresident)k
0inputs+0outputs (0major+322710minor)pagefaults 0swaps

Starting parsing benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt...
Parsing done, size: 261300000 in 4.53133 seconds
Number of iterations: 2
Number of states: 1300000, Number of partitions: 1300000
Computation took 5.50689 seconds
9.27user 0.78system 0:10.11elapsed 99%CPU (0avgtext+0avgdata 1158332maxresident)k
0inputs+0outputs (0major+289716minor)pagefaults 0swaps

Starting parsing benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt...
Parsing done, size: 13753169 in 0.21053101 seconds
Number of iterations: 186
Number of states: 1632799, Number of partitions: 357456
Computation took 26.238043 seconds
26.09user 0.35system 0:26.45elapsed 99%CPU (0avgtext+0avgdata 112876maxresident)k
0inputs+0outputs (0major+253470minor)pagefaults 0swaps

Starting parsing benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt...
Parsing done, size: 120061359 in 1.355099 seconds
Number of iterations: 7
Number of states: 4459455, Number of partitions: 4459455
Computation took 7.32517 seconds
8.05user 0.66system 0:08.73elapsed 99%CPU (0avgtext+0avgdata 1023292maxresident)k
0inputs+0outputs (0major+337009minor)pagefaults 0swaps