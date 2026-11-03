"""Generate the shared 8x8 toroidal void-and-cluster rank tile (no dependencies)."""
import math
import random
from pathlib import Path

SIDE = 8
COUNT = SIDE * SIDE
weights = [
    [math.exp(-sum(min(abs(a - b), SIDE - abs(a - b)) ** 2
                   for a, b in zip(divmod(i, SIDE), divmod(j, SIDE))) / (2 * 1.2 ** 2))
     for j in range(COUNT)] for i in range(COUNT)
]


def energy(pattern, i):
    return sum(weights[i][j] for j in pattern)


def ranks():
    pattern = set(random.Random(3917).sample(range(COUNT), COUNT // 2))
    for _ in range(10000):
        cluster = max(pattern, key=lambda i: (energy(pattern, i), -i))
        pattern.remove(cluster)
        hole = min(set(range(COUNT)) - pattern, key=lambda i: (energy(pattern, i), i))
        pattern.add(hole)
        if cluster == hole:
            break
    else:
        raise RuntimeError("void-and-cluster initialization did not converge")
    out = [0] * COUNT
    low = set(pattern)
    for rank in range(COUNT // 2 - 1, -1, -1):
        cluster = max(low, key=lambda i: (energy(low, i), -i))
        out[cluster] = rank
        low.remove(cluster)
    high = set(pattern)
    for rank in range(COUNT // 2, COUNT):
        hole = min(set(range(COUNT)) - high, key=lambda i: (energy(high, i), i))
        out[hole] = rank
        high.add(hole)
    return out


if __name__ == "__main__":
    tile = ranks()
    root = Path(__file__).parent
    root.joinpath("blue_noise.wgsl").write_text(
        "const BLUE_NOISE = array<u32,64>(" + ",".join(f"{v}u" for v in tile) + ");\n",
        encoding="utf-8", newline="\n")
    root.joinpath("blue_noise.hlsl").write_text(
        "static const uint BLUE_NOISE[64] = {" + ",".join(map(str, tile)) + "};\n",
        encoding="utf-8", newline="\n")
