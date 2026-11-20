"""Render the garage picker from Mashini's exported meshes in background Blender."""
import math
import struct
from pathlib import Path

import bpy
from mathutils import Vector

if not bpy.app.background:
    raise RuntimeError("Run this asset bake in background Blender")
bpy.ops.wm.read_factory_settings(use_empty=True)
assets = Path(__file__).resolve().parents[1] / "assets"
scene = bpy.context.scene
scene.render.engine = 'CYCLES'
scene.cycles.samples = 12
scene.render.film_transparent = True
scene.render.resolution_x = 720
scene.render.resolution_y = 600
scene.render.resolution_percentage = 100
scene.view_settings.view_transform = 'AgX'
scene.world = bpy.data.worlds.new("Studio fill")
scene.world.use_nodes = True
scene.world.node_tree.nodes['Background'].inputs[0].default_value = (0.75, 0.81, 0.92, 1)


def material(name, rgb, metal=0.0, rough=0.32):
    def linear(byte):
        v = byte / 255
        return v / 12.92 if v < 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    shader = m.node_tree.nodes.get("Principled BSDF")
    shader.inputs['Base Color'].default_value = tuple(linear((rgb >> n) & 255) for n in (16, 8, 0)) + (1,)
    shader.inputs['Metallic'].default_value = metal
    shader.inputs['Roughness'].default_value = rough
    return m


paints = [material("Enamel", col, .45, .20) for col in (0xc43b26, 0x1e847f, 0xf3ba47, 0x668ac1, 0xaa5880)]
materials = [None, material("Tyres", 0x232729), material("Trim", 0x58473e),
             material("Glass", 0xa5ced4, .1, .11), material("Chrome", 0xc7c9c6, .93, .20),
             material("Seats", 0xb36f41), material("Ivory", 0xe8d9ac),
             material("Lamp", 0xffedbb, .28), material("Tail light", 0xad1815)]
materials[3].node_tree.nodes['Principled BSDF'].inputs['Transmission Weight'].default_value = 0.74

bpy.ops.object.camera_add()
camera = bpy.context.object
camera.location = Vector((0.46, -1, .38)).normalized() * 24
camera.rotation_euler = (-camera.location).to_track_quat('-Z', 'Y').to_euler()
camera.data.type = 'ORTHO'
cell = 3.75
camera.data.ortho_scale = cell * 3
scene.camera = camera
rotation = camera.rotation_euler.to_matrix()
right, up = rotation @ Vector((1, 0, 0)), rotation @ Vector((0, 1, 0))

for position, power, size in [((3, -12, 15), 4300, 10), ((-9, 6, 11), 3600, 8), ((10, 8, 4), 2100, 5)]:
    bpy.ops.object.light_add(type='AREA', location=position)
    light = bpy.context.object
    light.data.energy, light.data.shape, light.data.size = power, 'DISK', size
    light.rotation_euler = (-light.location).to_track_quat('-Z', 'Y').to_euler()

for model in range(3):
    with (assets / f"car-{model}-2.mesh").open('rb') as f:
        for _ in range(struct.unpack('<I', f.read(4))[0]):
            group, mat, vertices, indices = struct.unpack('<4I', f.read(16))
            coords, normals = [], []
            for x, y, z, nx, ny, nz in struct.iter_unpack('<6f', f.read(24 * vertices)):
                coords.append((x, -z, y))
                normals.append((nx, -nz, ny))
            faces = list(struct.iter_unpack('<3H', f.read(2 * indices)))
            mesh = bpy.data.meshes.new("Die-cast component")
            mesh.from_pydata(coords, [], faces)
            mesh.materials.append(materials[mat] or paints[0])
            mesh.polygons.foreach_set('use_smooth', [True] * len(faces))
            mesh.normals_split_custom_set_from_vertices(normals)
            for paint, enamel in enumerate(paints):
                obj = bpy.data.objects.new("Garage preview", mesh)
                scene.collection.objects.link(obj)
                obj.location = right * ((model-1)*cell) + up * ((2-paint)*cell/2) - Vector((0, 0, .66))
                if mat == 0:
                    obj.material_slots[0].link = 'OBJECT'
                    obj.material_slots[0].material = enamel

scene.render.image_settings.file_format = 'PNG'
scene.render.image_settings.color_mode = 'RGBA'
scene.render.filepath = str(assets / "garage-models.png")
bpy.ops.render.render(write_still=True)
