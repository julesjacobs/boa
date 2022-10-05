import pathlib

def build_state_map(filename):
  f = open(filename)
  header = "#"
  while "#" in header: header = f.readline()
  n = 0
  state_map = dict()
  for line in f:
    parts = line.split(":")
    if len(parts)==1: continue
    state = parts[0]
    if state not in state_map:
      state_map[state] = n
      n += 1
  return state_map

import re
def parse(x):
  x = "(" + x + ")"
  x = re.sub("\{", "(", x)
  x = re.sub("\}", ",)", x)
  x = re.sub(":", ",", x)
  x = re.sub("([0-9]*s[0-9]*)", "\"\\1\"", x)
  y = eval(x)
  return y

def str_st(x):
  return "@"+str(state_map[x])

def str_tag(tag, content):
  return "[" + str(tag) + "]{" + content + "}"

numset = set()

factor = 2*2*2*2*3*3*5*7*11*13*17*19*23
def str_float(x):
  n = round(x*factor)
  if abs(n - x*factor) > 0.1:
    print("Not exactly representable in fixed point: " + str(x))
    exit()
  numset.add(n)
  return str(n)

# DX = R^X
def str_R(tag,x):
  return "Add" + str_tag(tag, ",".join(str_st(x[2*i]) + ":" + str_float(x[2*i+1]) for i in range(len(x)//2)))

def cost_R(tag,x):
  # return 1 #+1+(8+4)*(len(x)//2)-4
  # return 1+1+(8+4)*(len(x)//2)-4
  return 4+4+(8+4)*(len(x)//2)

# Nx(DX)
def str_NDX(x):
  (tag,x) = x
  return str_R(tag,x)

def cost_NDX(x):
  (tag,x) = x
  return cost_R(tag,x)

# P(Nx(DX))
def str_PNDX(tag,x):
  return "Set" + str_tag(tag, ",".join(str_NDX(a) for a in x))

def cost_PNDX(tag,x):
  # return sum(cost_NDX(a) for a in x)
  # return 1+1+sum(cost_NDX(a) for a in x)
  return 4+4+sum(cost_NDX(a) for a in x)

# 6 x P(Nx(DX))
def str_NPNDX(x):
  (tag,x) = x
  return str_PNDX(tag,x)

def cost_NPNDX(x):
  (tag,x) = x
  return cost_PNDX(tag,x)

def str_RX(x):
  return str_R(0,x)

def str_taglist(x):
  n = x[0]
  lst = x[1:]
  return "List"+str_tag(n,",".join(str_st(s) for s in lst))

def str_op_taglist(op,x):
  (tag,x) = x
  return op + str_tag(tag, ",".join(str_taglist(x[2*i]) + ":" + str(x[2*i+1]) for i in range(len(x)//2)))

def str_NWNXXX(x):
  return str_op_taglist("Or",x)

def str_ZZMNXXX(x):
  return str_op_taglist("Max",x)

def str_NPNXXX(x):
  (tag,x) = x
  return "Set" + str_tag(tag, ",".join(str_taglist(a) for a in x))

transl = {
  "6 x P(Nx(DX))": str_NPNDX,  # n*(1+1) + k*(1+1) + m*(8+4) - k*8
  "R^X": str_RX, # n*(1) + k*(8+4)
  "N × (Word, or)^(4×X×X×X×X×X)": str_NWNXXX,  # n*(1+1) + k*(8+1+r*4)
  "N × (Word, or)^(4×X×X×X×X)": str_NWNXXX,
  "N × (Word, or)^(4×X×X×X)": str_NWNXXX,
  "N × (Word, or)^(4×X×X)": str_NWNXXX,
  "N × (Word, or)^(4×X)": str_NWNXXX,
  "Z × (Z, max)^(4×X×X×X×X×X)": str_ZZMNXXX,  # n*(1+1) + k*(4+1+r*4)
  "Z × (Z, max)^(4×X×X×X×X)": str_ZZMNXXX,
  "Z × (Z, max)^(4×X×X×X)": str_ZZMNXXX,
  "Z × (Z, max)^(4×X×X)": str_ZZMNXXX,
  "Z × (Z, max)^(4×X)": str_ZZMNXXX,
  "2 × P(4×X×X×X×X×X)": str_NPNXXX, # n*(1+1) + k*(1+r*4)
  "2 × P(4×X×X×X×X)": str_NPNXXX,
  "2 × P(4×X×X×X)": str_NPNXXX,
  "2 × P(4×X×X)": str_NPNXXX,
  "2 × P(4×X)": str_NPNXXX,
}

# Rust functors:
# - u8
# - P(X)
# - R^X = DX
# - (Word, or)^X
# - (Z, max)^X
# - AxBxCxDxExF
#
# u32 × P(u32×X×X×X) --> n*(4+4) + k*4*4 bytes = 1300000*(4+4) + 65000000*4*4=1,050,400,000
# u8 × P(u8×X×X×X) --> n*(1+2) + k*(3*4+1) bytes = 1300000*(1+2) + 65000000*(3*4+1)=848,900,000
# difference = 1050400000 - 848900000=201,500,000
# k=65,000,000
# n=1,300,000
#
# 6 x P(Nx(DX))
# (0, {(0, {2s546: 1.0})}) -- 4+4+4+4+4=20  or  4+4+4+4=16  or  1+2+1+2+4=10
# n=1632799 (number of 6xP(Nx(DX)))
# k=2124442 (number of DX)
# m=5456481 (number of X)
# u32 x P(u32x(DX)) --> n*(4+4) + k*(4+4) + m*(8+4) = 1632799*(4+4) + 2124442*(4+4) + 5456481*(8+4)=95,535,700
# 1632799*4=6,531,196
# 2124442*4=8,497,768
# 5456481*4=21,825,924
# u8 x P(u8x(DX)) --> n*4 - k*4 + m*(8+4) = 1632799*4 - 2124442*4 + 5456481*(8+4)=63,511,200
#
# R^X
# n=4,459,455
# m=38,533,968
# n*4 + m*(4+8) = 4459455*4 + 38533968*(4+8)=480,245,436


files = list(pathlib.Path('benchmarks').glob('*/*.coalgebra'))
files = [str(file) for file in files]
# files = [str(file) for file in files if "large" in str(file)]

# exit()


for (k,filename) in enumerate(files):
  newfilename = str(filename).replace(".coalgebra", ".boa.txt")
  # newfilename = "foo.txt"
  f = open(filename)
  header = "#"
  while "#" in header: header = f.readline()
  # if header.strip() != "R^X": continue
  if header.strip() != "6 x P(Nx(DX))": continue

  print(k, filename, header)

  translfn = transl[header.strip()]

  state_map = build_state_map(filename)
  outf = open(newfilename, "w")

  # cost = 0
  n = 0
  for line in f:
    if line == "\n": continue
    (state,y) = parse(line)
    # cost += cost_NPNDX(y)
    if n != 0:
      outf.write("\n")
    n += 1
    outf.write(translfn(y))
    if n%10000 == 0:
      pass
      print(k, filename, header, n)
    # if n > 100: break
  print(filename, numset)
  print("Number of different numbers: ", len(numset))
  numset = set()
  # print("------------------")
  # print(filename, cost)
