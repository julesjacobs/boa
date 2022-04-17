for file in benchmarks/*/*.boa.txt; do
  ./target/release/boa convert ${file}
done