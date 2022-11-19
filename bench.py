import os
import glob
import re
import subprocess
import pprint
import datetime
import statistics
import math

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
  d['gtime_s'] = float(extract(r"User time \(seconds\): ([0-9]+.[0-9]+)",time_output))
  d['mem_mb'] = float(extract(r"Maximum resident set size \(kbytes\): ([0-9]+)",time_output))/1024

  stats = [
    (str, ['file', 'algorithm']),
    (float, ['size_mb', 'parsing_time_s', 'backrefs_time_s', 'selfreport_time_s', 'iter_time_s', 'coalg_refs_mb',  'refpart_mb']),
    (int, ['m_edges', 'iters', 'n_states', 'n_states_min'])
  ]
  for (fn,stattype) in stats:
    for stat in stattype:
      d[stat] = fn(extract(stat + ': (.+)\n',program_output))
  return d

def runmcrl2(algorithm="bisim"):
  def run(file):
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

    d['gtime_s'] = float(extract(r"User time \(seconds\): ([0-9]+.[0-9]+)",time_output))
    d['mem_mb'] = float(extract(r"Maximum resident set size \(kbytes\): ([0-9]+)",time_output))/1024
    d['selfreport_time_s'] = float(extract("reduction: ([0-9]+.[0-9]+)",time_output))
    return d
  return run

def runbench(folder, name, cmd, rep=1):
  outputfile = f"benchresults/rawdata/{name}_rep{rep}.txt"
  if os.path.exists(outputfile):
    df = eval(open(outputfile, "r").read())
    return df

  files = glob.glob(folder)
  benchmarks = [file for file in files for _ in range(rep)]
  df = []
  i = 0
  for file in benchmarks:
    i += 1
    print(f"Running benchmark {i} out of {len(benchmarks)} for benchmark set {name}")
    d = cmd(file)
    df.append(d)
    print(d)

  nows = datetime.datetime.now().strftime("%Y-%m-%d__%H:%M:%S")
  f = open(outputfile, "w")
  f.write(pprint.pformat(df))
  return df

def groupby(df, keyfn):
  out = dict()
  for row in df:
    k = keyfn(row)
    if k not in out: out[k] = dict()
    d = out[k]
    for key in row:
      if key not in d: d[key] = []
      d[key].append(row[key])
  return out

def filekeyfn(row):
  if "boa-file" in row:
    return row["boa-file"][:-3]
  if "mcrl-bisim-file" in row:
    return row["mcrl-bisim-file"][:-3]
  if "copar-dcpr-file" in row:
    return row["copar-dcpr-file"][:-3]
  raise Exception("Key missing!")

def prefixkeys(prefix, df):
  return [dict((prefix+key,val) for key,val in row.items()) for row in df]

def merge(df1, df2):
  keys1 = set(df1.keys())
  keys2 = set(df2.keys())
  if keys1 != keys2:
    raise Exception(f"Different key sets: \n {keys1=} \n\n {keys2=}")
  return dict((key, df1[key]|df2[key]) for key in set(df1.keys()) | set(df2.keys()))

################################################################
# Run the benchmarks or get them from benchresults/*.txt files #
################################################################

reps = 1
os.system("cargo build -r")
boa_coalg = runbench("benchmarks/coalg/*/*.boa", 'boa-coalg', runboa, reps)
copar_dcpr_coalg = runbench("benchmarks/coalg/*/*.boa", 'copar-dcpr-coalg', exit, 1)
boa_lts = runbench("benchmarks/lts/*/*.boa", 'boa-lts', runboa, reps)
mcrl_bisim_lts = runbench("benchmarks/lts/*/*.aut", 'mcrl2-bisim-lts', runmcrl2("bisim"), reps)

## Slower algorithms ##
# mcrl_bisim_gv_lts = runbench("benchmarks/lts/*/*.aut", 'mcrl2-bisim-lts', runmcrl2("bisim-gv"), 2)
# mcrl_bisim_gjkw_lts = runbench("benchmarks/lts/*/*.aut", 'mcrl2-bisim-gjkw-lts', runmcrl2("bisim-gjkw"), 2)
# mcrl_bisim_sig_lts = runbench("benchmarks/lts/*/*.aut", 'mcrl2-bisim-sig-lts', runmcrl2("bisim-sig"), 2)

# Preprocess the data
boa_coalg = groupby(prefixkeys("boa-", boa_coalg), filekeyfn)
copar_dcpr_coalg = groupby(prefixkeys("copar-dcpr-", copar_dcpr_coalg), filekeyfn)
boa_lts = groupby(prefixkeys("boa-", boa_lts), filekeyfn)
mcrl_bisim_lts = groupby(prefixkeys("mcrl-bisim-", mcrl_bisim_lts), filekeyfn)

# The data for the 2 tables
coalg = merge(boa_coalg,copar_dcpr_coalg)
lts = merge(boa_lts, mcrl_bisim_lts)

#####################
# Make latex tables #
#####################

typetransl = {
  'vasy': 'vasy',
  'cwi': 'cwi',
  'wta_Z': 'wta(Z)',
  'wta_powerset': 'wta(2)',
  'wta_Word': 'wta(W)',
  'wlan': 'wlan',
  'fms': 'fms'
}

def get_type(row):
  f = row['boa-file'][0]
  for k,v in typetransl.items():
    if k in f: return v
  raise Exception(f"Type not found in {f=}")

def mktimefmt(m):
  def timefmt(values):
    if None in values: return "\\tna"
    mt = max(statistics.mean(values),0.0001)
    # mean = "{:.2f}".format(m/mt/1e6,2)
    if float(round(mt)) == mt:
      mean = str(int(mt))
    else:
      mean = "{:.2f}".format(mt,2)
    # stdev = "{:.2f}".format(max(statistics.stdev(values), 0.01))
    return f"{mean}" # $\pm$ {stdev}"
  return timefmt

def memfmt(values):
  if None in values: return "\\tna"
  return str(round(statistics.mean(values)))

def row_coalg(row):
  type = get_type(row)
  if "wta" in type:
    typefmt = type.replace("wta", f"wta$_{row['copar-dcpr-r'][0]}$")
  else:
    typefmt = type
  n = row['boa-n_states'][0]
  n_min = row['boa-n_states_min'][0]
  m = row['boa-m_edges'][0]

  boa_times = row['boa-gtime_s']
  copar_times = row['copar-dcpr-copar_time_s']
  dcpr_times = row['copar-dcpr-dcpr_time_s']

  boa_mems = row['boa-mem_mb']
  # copar_mems = row['copar-dcpr-copar_mem_mb']
  dcpr_mems = row['copar-dcpr-dcpr_mem_mb']

  timefmt = mktimefmt(m)

  return {
    'type': type,
    'typefmt': typefmt,
    'n': n,
    'perc_red': str(math.floor(100*(n - n_min)/n))+"\%",
    'n_min': n_min,
    'm': m,
    'copar_times': timefmt(copar_times),
    'dcpr_times': timefmt(dcpr_times),
    'boa_times': timefmt(boa_times),
    # 'copar_mems': '\\approx 16000',
    # 'copar_mems': '$>$160000' if None in copar_times else '$\\approx$160000',
    'dcpr_mems': str(dcpr_mems[0]) + "\\tnodes",
    'boa_mems': memfmt(boa_mems),
    # 'n_per_sec': round(n / max(statistics.mean(boa_times),0.0001) / 1e6, 2),
    # 'm_per_sec': round(m / statistics.mean(boa_times) / 1e6, 2),
    'k': round(m/n,2),
  }

def row_lts(row):
  type = get_type(row)
  n = row['boa-n_states'][0]
  n_min = row['boa-n_states_min'][0]
  m = row['boa-m_edges'][0]

  boa_times = row['boa-gtime_s']
  mcrl_times = row['mcrl-bisim-selfreport_time_s']

  boa_mems = row['boa-mem_mb']
  mcrl_mems = row['mcrl-bisim-mem_mb']

  timefmt = mktimefmt(m)

  return {
    'type': type,
    'typefmt': type,
    'n': n,
    'perc_red': str(math.floor(100*(n - n_min)/n))+"\%",
    'n_min': n_min,
    'm': m,
    'mcrl_times': timefmt(mcrl_times),
    'boa_times': timefmt(boa_times),
    # 'copar_mems': '\\approx 16000',
    'mcrl_mems': memfmt(mcrl_mems),
    'boa_mems': memfmt(boa_mems),
    # 'n_per_sec': round(n / max(statistics.mean(boa_times),0.0001) / 1e6, 2),
    # 'm_per_sec': round(m / max(statistics.mean(boa_times),0.0001) / 1e6, 2),
    'k': round(m/n,2),
  }


def printtable(data):
  data = sorted(data, key=lambda r: (r['type'], r['n']))
  row0 = data[0].keys()
  sep = lambda vs: " & ".join([str(v).rjust(15) for v in list(vs)[1:]])+" \\\\"
  out = [sep(row0)]
  lasttype = None
  for row in data:
    if float(row['boa_times'].split()[0]) < 0.02: continue
    if row['type'] != lasttype:
      if lasttype:
        out.append("\\midrule")
      else:
        out.append("\\toprule")
    lasttype = row['type']
    out.append(sep(row.values()))
  out.append("\\bottomrule")
  return "\n".join(out)

outS = f"Repetitions: {reps}"
outS += "\n"*3

coalgT = [row_coalg(r) for r in coalg.values()]
outS += printtable(coalgT)
outS += "\n"*3

ltsT = [row_lts(r) for r in lts.values()]
outS += printtable(ltsT)

print(outS)
out = open(f"benchresults/latextables/tables_{reps}_reps.tex", "w")
out.write(outS)





# \begin{tabular}{>{\bfseries}c>{\bfseries}r@{\hskip 1.5cm}rrr@{\hskip 1.5cm}rr}
#   % \toprule
#                         \multicolumn{2}{l}{\thead{benchmark}}         &\multicolumn{3}{l}{\thead{time (s)}} & \multicolumn{2}{c}{\thead{memory (MB)}} \\ \toprule
#   \thead{type}            &\thead{n}&  \copar{} & \distr{} &   \ours{} &        \distr{} &     \ours{} \\
#   \toprule    \tc{5}{fms} & 35910   &      4 &     2 &   0.01 &    13\tnodes &     6 \\
#                           & 152712  &     17 &     8 &   0.08 &    62\tnodes &    20 \\
#                           & 537768  &     68 &    26 &   0.41 &   163\tnodes &    71 \\
#                           & 1639440 &    232 &    84 &   1.12 &   514\tnodes &   196 \\
#                           & 4459455 &   \tna &   406 &   4.47 &  1690\tnodes &   582 \\
#   \midrule   \tc{3}{wlan} & 248503  &     39 &   297 &   0.11 &    90\tnodes &    15 \\
#                           & 607727  &    105 &   855 &   0.28 &   147\tnodes &    42 \\
#                           & 1632799 &   \tna &  2960 &   0.79 &   379\tnodes &    93 \\
#   \midrule \tc{6}{wta(W)} & 83431   &    642 &    52 &   0.76 &   663\tnodes &   143 \\
#                           & 92615   &    511 &    61 &   1.14 &   849\tnodes &   194 \\
#                           & 94425   &    528 &    59 &   0.73 &   639\tnodes &   124 \\
#                           & 134082  &    471 &    76 &   0.91 &   675\tnodes &   125 \\
#                           & 152107  &    566 &    79 &   0.74 &   642\tnodes &    83 \\
#                           & 944250  &   \tna &   675 &  11.96 &  6786\tnodes &  1228 \\
#   \midrule \tc{6}{wta(Z)} & 92879   &    463 &    56 &   0.66 &   754\tnodes &   161 \\
#                           & 94451   &    445 &    61 &   0.80 &   871\tnodes &   200 \\
#                           & 100799  &    391 &    64 &   0.59 &   628\tnodes &   135 \\
#                           & 118084  &    403 &    74 &   0.61 &   633\tnodes &   113 \\
#                           & 156913  &    438 &    82 &   0.48 &   677\tnodes &    92 \\
#                           & 1007990 &   \tna &   645 &  16.75 &  5644\tnodes &  1325 \\
#   \midrule \tc{6}{wta(2)} & 86852   &    537 &    71 &   0.84 &   701\tnodes &   178 \\
#                           & 92491   &    723 &    67 &   0.81 &   728\tnodes &   154 \\
#                           & 134207  &    689 &   113 &   0.95 &   825\tnodes &   175 \\
#                           & 138000  &    467 &   129 &   0.92 &   715\tnodes &   124 \\
#                           & 154863  &    449 &   160 &   0.81 &   621\tnodes &    79 \\
#                           & 1300000 &   \tna &  1377 &  23.35 &  7092\tnodes &  1647 \\
# \bottomrule
# \end{tabular}


# \begin{tabular}{>{\bfseries}c>{\bfseries}r>{\bfseries}r@{\hskip 1.5cm}rr@{\hskip 1.5cm}rr}
#   % \toprule
#                         \multicolumn{3}{l}{\thead{benchmark}}         &\multicolumn{2}{l}{\thead{time (s)}} & \multicolumn{2}{c}{\thead{memory (MB)}} \\ \toprule
#   \thead{type}            &\thead{n} &\thead{n$_{min}$} & \mcrl{} &   \ours{} &        \mcrl{} &     \ours{} \\
#   \toprule    \tc{5}{cwi} & 566640    & 15518   & 5.3   & 0.4  & 408   & 58 \\
#                           & 2165446   & 31906   & 9.6   & 1.4  & 978   & 164 \\
#                           & 2416632   & 95610   & 15.0  & 1.4  & 1772  & 249 \\
#                           & 7838608   & 966470  & 221.7 & 15.8 & 5777  & 814 \\
#                           & 33949609  & 122035  & 281.3 & 31.5 & 16673 & 2776 \\
#   \midrule  \tc{12}{vasy} & 164865    & 1136    & 1.7   & 0.2  & 162   & 23 \\
#                           & 66929     & 66929   & 2.3   & 0.1  & 275   & 18 \\
#                           & 65537     & 65536   & 5.8   & 0.1  & 554   & 28 \\
#                           & 1112490   & 265     & 8.7   & 0.7  & 579   & 94 \\
#                           & 6120718   & 5199    & 15.1  & 2.2  & 1297  & 326 \\
#                           & 574057    & 3577    & 16.6  & 2.1  & 1278  & 141 \\
#                           & 2581374   & 2581374 & 28.1  & 1.7  & 2691  & 274 \\
#                           & 4220790   & 1356477 & 32.9  & 2.5  & 2068  & 312 \\
#                           & 6020550   & 7168    & 32.3  & 3.1  & 2124  & 521 \\
#                           & 4338672   & 2581374 & 37.4  & 2.9  & 3085  & 350 \\
#                           & 1102693   & 882341  & 53.6  & 6.1  & 2768  & 620 \\
#                           & 1232370   & 996774  & 59.1  & 7.0  & 3103  & 734 \\
#                           & 8082905   & 408     & 70.0  & 3.6  & 4313  & 732 \\
# \bottomrule
# \end{tabular}