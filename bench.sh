cargo build -r

for file in benchmarks/large/*.boa; do
  gtime ./target/release/boa naive ${file}; echo ""
  gtime ./target/release/boa nlogn ${file}; echo ""
done

gtime ./target/release/boa naive benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa; echo ""
gtime ./target/release/boa nlogn benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa; echo ""

gtime ./target/release/boa naive benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa; echo ""
gtime ./target/release/boa nlogn benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa; echo ""

# gtime ./target/release/boa benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt