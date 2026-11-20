"""Bake a damped shallow-water impact into StarHermit's WABC Alembic mesh cache."""
import hashlib
import json
import math
from pathlib import Path
import struct
import zlib


def bake(size, tier):
    points = []
    triangles = []
    for j in range(size):
        z = 2 * j / (size - 1) - 1
        for i in range(size):
            x = 2 * i / (size - 1) - 1
            # Map the reflecting square grid onto the circular bucket wall.
            points.append((x * math.sqrt(1 - z*z/2), 0, z * math.sqrt(1 - x*x/2)))
            k = j*size + i
            if i and j:
                triangles.extend((k-size-1, k-1, k, k-size-1, k, k-size))
    chunks = bytearray()

    def chunk(raw, **metadata):
        packed = zlib.compress(raw, 9)
        entry = dict(offset=len(chunks), length=len(packed), **metadata)
        chunks.extend(packed)
        return entry

    base = chunk(b''.join(struct.pack('<3f', *p) for p in points)
                 + struct.pack(f'<{len(triangles)}H', *triangles),
                 vertexCount=len(points), indexCount=len(triangles))
    heights = [0.] * len(points)
    velocities = [0.] * len(points)
    neighbors = [(k-1 if k%size else k, k+1 if k%size<size-1 else k,
                  k if k<size else k-size, k+size if k+size<len(points) else k)
                 for k in range(len(points))]
    impulse = []
    for x, _, z in points:
        r2 = ((x-0.39)/0.24)**2 + (z/0.30)**2
        impulse.append(13*(r2-0.70)*math.exp(-r2))
    frames = []
    step = 1/120
    for frame in range(73):
        fade = min(1, (72-frame)/9)
        layer = b''.join(struct.pack('<4f', 0, round(h*fade, 5), 0, 0) for h in heights)
        frames.append(chunk(layer, vertexCount=len(points)))
        for substep in range(4):
            for k, (l,r,d,u) in enumerate(neighbors):
                lap = heights[l]+heights[r]+heights[d]+heights[u]-4*heights[k]
                velocities[k] += step * (lap*(0.85*(size-1)/2)**2 - 2.7*velocities[k])
                if frame==0 and substep==0:
                    velocities[k] += impulse[k]
            heights = [h + step*v for h,v in zip(heights, velocities)]
    geometry = dict(mode='vertex', normals='smooth', numSamples=len(frames), timeSampling=0,
                    layout=dict(stride=12, attributes=[dict(name='position', format='f32x3', offset=0)]),
                    base=base, animation=dict(format='f32x4', texelStride=1, width=size, height=size),
                    samples=frames)
    source = Path(__file__)
    header = dict(writer='Mashini shallow-water bake', source=dict(file=source.name,sha256=hashlib.sha256(source.read_bytes()).hexdigest()),
                  compression='deflate', space='world', timeSamplings=[dict(type='uniform',start=0,step=1/30)],
                  nodes=[dict(name='Water',path='/Water',parent=-1,type='mesh',geometry=geometry)])
    meta = json.dumps(header,separators=(',',':')).encode()
    (source.resolve().parents[1]/'assets'/f'bucket-{tier}.wabc').write_bytes(
        b'WABC\x01\0\0\0'+struct.pack('<I',len(meta))+meta+chunks)


if __name__ == '__main__':
    for tier, size in enumerate((17, 31, 49)):
        bake(size, tier)
