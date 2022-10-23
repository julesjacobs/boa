import os
import glob
import csv
import re
import subprocess


# Try to find a working time command
try:
  out, err = subprocess.Popen(["gtime", "-v", "echo", "hi"], stdout=subprocess.PIPE, stderr=subprocess.PIPE).communicate()
  time_command = "gtime"
except FileNotFoundError:
  try:
    out, err = subprocess.Popen(["/usr/bin/time", "-v", "echo", "hi"], stdout=subprocess.PIPE, stderr=subprocess.PIPE).communicate()
    time_command = "/usr/bin/time"
  except FileNotFoundError:
    print("Either gtime -v or /usr/bin/time -v must work to determine memory usage.")
    print("On Linux you should already have /usr/bin/time.")
    print("On OS X you can install gtime using `brew install gnu-time`.")
    print("Benchmarking likely does not work on Windows.")
    exit(1)

executable = "ltsconvert"

# files = glob.glob("benchmarks/*/*.boa")
files = glob.glob("ltsbenchmarks/*/*/*.aut")
algs = ["bisim", "bisim-gv", "bisim-gjkw", "bisim-sig"]
# algs = ["bisim"]
benchmarks = [(file, algorithm) for file in files for algorithm in algs for _ in range(3)]


w = csv.writer(open("benchresultsmcrl2_rep456.csv", "w"))
w.writerow(["file", "algorithm", "selfreport", "compressedsize_mb", "mem_mb", "time_sec"])


for (file,algorithm) in benchmarks:
  print((file,algorithm))
  try:
    out, err = subprocess.Popen([time_command, "-v", executable, "--timings", "--equivalence="+algorithm, file],
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE).communicate(timeout=500)
  except:
    print("timeout")
    w.writerow([file,algorithm, 0, 0, 0, 0])
    continue


  program_output = out.decode("utf-8")
  time_output = err.decode("utf-8")

  time = re.findall(r"User time \(seconds\): ([0-9]+.[0-9]+)",time_output,re.MULTILINE)
  if len(time) != 1:
    print("Problem with time output (no user time):")
    print(time_output)
    exit()
  time_sec = float(time[0])

  mem = re.findall(r"Maximum resident set size \(kbytes\): ([0-9]+)",time_output,re.MULTILINE)
  if len(mem) != 1:
    print("Problem with time output (no maximum resident set size):")
    print(time_output)
    exit()
  mem = int(mem[0])
  mem_mb = float(mem)/1024

  compressedsize = os.path.getsize(file)
  compressedsize_mb = float(compressedsize)/(1024*1024)

  selfreport = re.findall(r"reduction: ([0-9]+.[0-9]+)",time_output,re.MULTILINE)
  if len(selfreport) != 1:
    print("Problem with selfreported reduction output:")
    print(program_output)
    exit()
  selfreport = float(selfreport[0])

  w.writerow([file,algorithm,selfreport,compressedsize_mb, mem_mb, time_sec])

  print(f"{file=}, {algorithm=}, {selfreport=}, {compressedsize_mb=}, {mem_mb=}, {time_sec=}")