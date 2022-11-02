import os
import glob
import re
import subprocess
import pprint
import datetime

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


def extract(keyre, outp):
  x = re.findall(keyre, outp,re.MULTILINE)
  if len(x) != 1:
    print(f"Problem with program output for regex {keyre=}:")
    print(outp)
    exit()
  return x[0]

def runboa(file):
  executable = "./target/release/boa"
  out, err = subprocess.Popen([time_command, "-v", executable, 'nlogn', file],
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE).communicate()
  program_output = out.decode("utf-8")
  time_output = err.decode("utf-8")

  d = dict()
  d['file'] = file
  d['gtime_s'] = extract(r"User time \(seconds\): ([0-9]+.[0-9]+)",time_output)
  d['mem_mb'] = str(float(extract(r"Maximum resident set size \(kbytes\): ([0-9]+)",time_output))/1024)

  stats = [
    (str, ['file', 'algorithm']),
    (float, ['size_mb', 'parsing_time_s', 'backrefs_time_s', 'reduction_time_s', 'coalg_refs_mb',  'refpart_mb']),
    (int, ['m_edges', 'iters', 'n_states', 'n_states_min'])
  ]
  for (fn,stattype) in stats:
    for stat in stattype:
      d[stat] = fn(extract(stat + ': (.+)\n',program_output))
  return d

def runmcrl2(file):
  algorithm = 'bisim'
  executable = "ltsconvert"
  d = dict()
  d['file'] = file
  d['algorithm'] = algorithm
  try:
    out, err = subprocess.Popen([time_command, "-v", executable, "--timings", "--equivalence="+algorithm, file],
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE).communicate(timeout=500)
  except:
    print("timeout")
    d['timeout'] = True
    return d

  # program_output = out.decode("utf-8")
  time_output = err.decode("utf-8")

  d = dict()
  d['gtime_s'] = extract(r"User time \(seconds\): ([0-9]+.[0-9]+)",time_output)
  d['mem_mb'] = str(float(extract(r"Maximum resident set size \(kbytes\): ([0-9]+)",time_output))/1024)
  d['time_selfreport_s'] = float(extract("reduction: ([0-9]+.[0-9]+)",time_output))
  return d

def runbench(folder, name, cmd, rep=1):
  files = glob.glob(folder)
  benchmarks = [file for file in files for _ in range(rep)]
  df = []
  i = 0
  for file in benchmarks:
    print(f"Running benchmark {i} out of {len(benchmarks)} for benchmark set {name}")
    i += 1
    d = cmd(file)
    df.append(d)
    print(d)

  nows = datetime.datetime.now().strftime("%Y-%m-%d__%H:%M:%S")
  f = open(f"benchresults/boa-{name}__{nows}.txt", "w")
  f.write(pprint.pformat(df))

os.system("cargo build -r")
runbench("benchmarks/*/*.boa", 'coalg', runboa, 10)
runbench("ltsbenchmarks/*/*/*.boa", 'lts', runboa, 10)
runbench("ltsbenchmarks/*/*/*.aut", 'lts', runmcrl2, 10)