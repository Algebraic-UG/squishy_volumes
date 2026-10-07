import sys
from pathlib import Path

import bpy
from nodebpy import geometry as g
from nodebpy.builder import (
    BooleanSocket,
    CustomGeometryGroup,
)
from nodebpy.nodes.geometry import StoreNamedAttribute
from nodebpy.types import (
    InputGeometry,
    InputInteger,
    InputVector,
)

scripts_dir = Path(sys.argv[0]).parent
repo_root = scripts_dir / ".."
asset_dir = repo_root / "python" / "src" / "squishy_volumes_extension" / "assets"
asset_file = asset_dir / "assets.blend"


class GenerateGrid(CustomGeometryGroup):
    _name = "Generate Grid"

    def __init__(self):
        super().__init__()

    def _build_group(self, tree):
        geometry = tree.inputs.geometry()
        sep = geometry >> g.GetGeometryBundle() >> g.SeparateBundle()

        spacing = sep.items.float("Spacing")
        random = sep.items.float("Random")

        with g.Frame("Calculate Extents") as _:
            bb = geometry >> g.BoundingBox()
            extents = g.VectorMath.subtract()
            bb.o.max >> extents.i.vector
            bb.o.min >> extents.i.vector_001

        with g.Frame("Point Count, Total and Along XYZ") as _:
            xyz = (
                g.Math.divide(
                    value=1.0,
                    value_001=spacing,
                )
                >> g.VectorMath.scale(extents)
                >> g.SeparateXYZ()
            )

            floor_and_max = lambda c: (
                c
                >> g.FloatToInteger(rounding_mode="FLOOR")
                >> g.IntegerMath.maximum(value_001=1)
            )

            x = floor_and_max(xyz.o.x)
            y = floor_and_max(xyz.o.y)
            z = floor_and_max(xyz.o.z)

            xyz = g.CombineXYZ(x, y, z)

            total = g.IntegerMath.multiply(g.IntegerMath.multiply(x, y), z)

        with g.Frame("Normalized Position From Index") as _:
            i = g.Index()
            normalized_position = g.CombineXYZ(
                g.IntegerMath.divide_floor(i, g.IntegerMath.multiply(y, z)),
                g.IntegerMath.modulo(g.IntegerMath.divide_floor(i, z), y),
                g.IntegerMath.modulo(i, z),
            )
        with g.Frame("Add Radomness") as _:
            normalized_position = g.VectorMath.add(
                normalized_position,
                g.RandomValue.vector(
                    min=random >> g.VectorMath.scale(vector=(-1,) * 3),
                    max=random >> g.VectorMath.scale(vector=(1,) * 3),
                ),
            )

        with g.Frame("Denormalize Position") as _:
            redivided = g.VectorMath.divide(
                extents,
                xyz,
            )

            position = g.VectorMath.multiply_add(
                redivided,
                normalized_position,
                g.VectorMath.multiply_add(
                    redivided,
                    (0.5,) * 3,
                    bb.o.min,
                ),
            )

        with g.Frame("Generate Points") as _:
            (
                g.Points(
                    total,
                    position,
                )
                >> g.PointsToVertices()
                >> tree.outputs.geometry()
            )


class IsInsideObject(CustomGeometryGroup):
    _name = "Is Inside Object"

    def __init__(
        self,
        geometry: InputGeometry = ...,
        position: InputVector = (0.0,) * 3,
        seed: InputInteger = 0,
    ):
        super().__init__(Geometry=geometry, Position=position, Seed=seed)

    def _build_group(self, tree):
        geometry = tree.inputs.geometry("Geometry")
        bb = geometry >> g.BoundingBox()
        extents = g.VectorMath.subtract()
        bb.o.max >> extents.i.vector
        bb.o.min >> extents.i.vector_001
        xyz = extents >> g.SeparateXYZ()

        dir = g.RandomValue.vector(
            min=(-1.0,) * 3, max=(1.0,) * 3, seed=tree.inputs.integer("Seed")
        )
        raycast = g.Raycast(
            target_geometry=geometry,
            source_position=tree.inputs.vector("Position"),
            ray_direction=dir,
            ray_length=g.Math.add(g.Math.add(xyz.o.x, xyz.o.y), xyz.o.z),
        )

        g.BooleanMath.l_and(
            raycast.o.is_hit,
            g.Compare.float.greater_than(
                a=g.VectorMath.dot_product(raycast.o.hit_normal, dir),
                b=0.0,
            ),
        ) >> tree.outputs.boolean("Inside")


class SampleParticles(CustomGeometryGroup):
    _name = "Sample Particles"

    def __init__(self):
        super().__init__()

    def _build_group(self, tree):
        geometry = tree.inputs.geometry()
        position = g.Position()

        (
            geometry
            >> GenerateGrid()
            >> g.DeleteGeometry(
                selection=g.BooleanMath.l_and(
                    IsInsideObject(geometry, position, 0),
                    IsInsideObject(geometry, position, 1),
                )
                >> g.BooleanMath.l_not()
            )
            >> tree.outputs.geometry()
        )


class Record(CustomGeometryGroup):
    _name = "Particle Parameters"

    def __init__(self):
        super().__init__()

    def _build_group(self, tree):
        geometry = tree.inputs.geometry()

        items = (
            (geometry >> g.GetGeometryBundle()).o.bundle >> g.SeparateBundle()
        ).items
        initial_velocity_linear = items.vector("Initial Velocity Linear")
        initial_velocity_angular = items.vector("Initial Velocity Angular")
        density = items.float("Density")
        viscosity_dynamic = items.float("Viscosity Dynamic")
        viscosity_bulk = items.float("Viscosity Bulk")
        youngs_modulus = items.float("Young's Modulus")
        poissons_ratio = items.float("Poissons's Ratio")
        sand_alpha_value = items.float("Sand Alpha Value")
        bulk_modulus = items.float("Bulk Modulus")
        exponent = items.integer("Exponent")
        material_type = items.integer("Type")
        viscosity = items.boolean("Viscosity")
        sand_alpha = items.boolean("Sand Alpha")
        size = items.float("Size")

        with g.Frame("Object Transform") as _:
            info = g.SelfObject() >> g.ObjectInfo()
            geometry = geometry >> StoreNamedAttribute.point.matrix(
                name="squishy_volumes_transform",
                value=g.CombineTransform(
                    g.TransformPoint(g.Position(), info.o.transform),
                    info.o.rotation,
                    info.o.scale,
                ),
            )

        with g.Frame("Initial Velocity") as _:
            geometry = geometry >> StoreNamedAttribute.point.vector(
                name="squishy_volumes_initial_velocity",
                value=g.VectorMath.add(
                    initial_velocity_linear,
                    g.VectorMath.cross_product(
                        initial_velocity_angular,
                        g.TransformDirection(
                            g.Position(),
                            (g.SelfObject() >> g.ObjectInfo()).o.transform,
                        ),
                    ),
                ),
            )

        with g.Frame("Common Parameters") as _:
            geometry = (
                geometry
                >> StoreNamedAttribute.point.float(
                    name="squishy_volumes_size", value=size
                )
                >> StoreNamedAttribute.point.float(
                    name="squishy_volumes_density", value=density
                )
                >> StoreNamedAttribute.point.float(
                    name="squishy_volumes_viscosity_dynamic", value=viscosity_dynamic
                )
                >> StoreNamedAttribute.point.float(
                    name="squishy_volumes_viscosity_bulk", value=viscosity_bulk
                )
            )

        with g.Frame("Solid Parameters") as _:
            solid = (
                geometry
                >> StoreNamedAttribute.point.float(
                    name="squishy_volumes_youngs_modulus",
                    value=youngs_modulus,
                )
                >> StoreNamedAttribute.point.float(
                    name="squishy_volumes_poissons_ratio",
                    value=poissons_ratio,
                )
                >> StoreNamedAttribute.point.float(
                    name="squishy_volumes_sand_alpha_value",
                    value=sand_alpha_value,
                )
            )

        with g.Frame("Fluid Parameters") as _:
            fluid = (
                geometry
                >> StoreNamedAttribute.point.float(
                    name="squishy_volumes_bulk_modulus",
                    value=bulk_modulus,
                )
                >> StoreNamedAttribute.point.integer(
                    name="squishy_volumes_exponent",
                    value=exponent,
                )
            )

        geometry = g.IndexSwitch.geometry(material_type, [solid, fluid])

        with g.Frame("Flags") as _:
            geometry = (
                geometry
                >> g.StoreNamedAttribute.point.boolean(
                    name="squishy_volumes_is_solid",
                    value=g.Compare.integer.equal(0, material_type),
                )
                >> g.StoreNamedAttribute.point.boolean(
                    name="squishy_volumes_is_fluid",
                    value=g.Compare.integer.equal(1, material_type),
                )
                >> g.StoreNamedAttribute.point.boolean(
                    name="squishy_volumes_use_vicosity",
                    value=viscosity,
                )
                >> g.StoreNamedAttribute.point.boolean(
                    name="squishy_volumes_use_sand_alpha",
                    value=sand_alpha,
                )
            )

        geometry >> tree.outputs.geometry()


def generate_particles() -> str:
    with g.tree("Squishy Volumes Generate Particles", split_inputs=True) as tree:
        with tree.inputs.panel("Sampling"):
            spacing = g.Math.divide(
                value=tree.inputs.float("Grid Node Size", default_value=0.5),
                value_001=tree.inputs.float("Sampling Factor", default_value=2.0),
            )
            sampling_bundle = g.CombineBundle(
                {
                    "Spacing": spacing,
                    "Random": tree.inputs.float("Random", default_value=0.5),
                }
            )

        points = (
            tree.inputs.geometry()
            >> g.SetGeometryBundle(bundle=sampling_bundle)
            >> SampleParticles()
        )

        parameters = dict()
        parameters["Size"] = spacing

        with tree.inputs.panel("Initial Parameters"):
            start_frame = tree.inputs.integer("Start Frame", default_value=1)

            parameters["Density"] = tree.inputs.float("Density", default_value=1000.0)
            type_switch = tree.inputs.menu(name="Type") >> g.MenuSwitch.integer()
            is_solid = type_switch.add_item("Solid", 0).output
            is_fluid = type_switch.add_item("Fluid", 1).output

            assert isinstance(is_solid, BooleanSocket)
            assert isinstance(is_fluid, BooleanSocket)

            parameters["Type"] = type_switch.o.output
            parameters["Young's Modulus"] = is_solid.switch.float(
                true=tree.inputs.float("Young's Modulus", default_value=10000.0)
            )
            parameters["Poissons's Ratio"] = is_solid.switch.float(
                true=tree.inputs.float("Poisson's Ratio", default_value=0.3)
            )
            parameters["Bulk Modulus"] = is_fluid.switch.float(
                true=tree.inputs.float("Bulk Modulus", default_value=1000.0)
            )
            parameters["Exponent"] = is_fluid.switch.integer(
                true=tree.inputs.integer("Exponent", default_value=2)
            )

            with tree.inputs.panel(name="Viscosity") as _:
                viscosity = tree.inputs.boolean(name="Viscosity", is_panel_toggle=True)
                parameters["Viscosity"] = viscosity
                parameters["Viscosity Dynamic"] = viscosity.switch.float(
                    true=tree.inputs.float("Viscosity Dynamic", default_value=3.0)
                )
                parameters["Viscosity Bulk"] = viscosity.switch.float(
                    true=tree.inputs.float("Viscosity Bulk", default_value=3.0)
                )

            with tree.inputs.panel(name="Sand Alpha") as _:
                sand_alpha = tree.inputs.boolean(
                    name="Sand Alpha", is_panel_toggle=True
                )
                parameters["Sand Alpha"] = sand_alpha
                parameters["Sand Alpha Value"] = sand_alpha.switch.float(
                    true=tree.inputs.float("Sand Alpha Value", default_value=0.3)
                )

            with tree.inputs.panel(name="Initial Velocity") as _:
                initial_velocity = tree.inputs.boolean(
                    name="Initial Velocity", is_panel_toggle=True
                )
                parameters["Initial Velocity"] = initial_velocity
                parameters["Initial Velocity Linear"] = initial_velocity.switch.vector(
                    true=tree.inputs.vector("Initial Velcoity Linear")
                )
                parameters["Initial Velocity Angular"] = initial_velocity.switch.vector(
                    true=tree.inputs.vector("Initial Velcoity Angular")
                )

        (
            (
                g.SceneTime().o.frame >> g.Compare.integer.equal(b=start_frame)
            ).o.result.switch.geometry(
                false=points,
                true=points
                >> g.SetGeometryBundle(bundle=g.CombineBundle(parameters))
                >> Record(),
            )
            >> tree.outputs.geometry()
        )

    tree.tree.is_modifier = True
    return tree.tree.name


def set_goals() -> str:
    with g.tree("Squishy Volumes Set Goals", split_inputs=True) as tree:
        geometry = tree.inputs.geometry()
        choose = tree.inputs.object("Choose")
        move = tree.inputs.object("Move")

        with g.Frame(label="Detect Choosen Particles") as _:
            choose_geometry = (
                choose >> g.ObjectInfo(transform_space="RELATIVE")
            ).o.geometry
            position = g.Position()
            inside = g.BooleanMath.l_and(
                IsInsideObject(geometry=choose_geometry, position=position, seed=0),
                IsInsideObject(geometry=choose_geometry, position=position, seed=1),
            )

        with g.Frame(label="Local -> Chooser -> Mover -> Back to Local") as _:
            goal_position = g.TransformPoint(
                g.Position(),
                g.MultiplyMatrices(
                    g.MultiplyMatrices(
                        (g.SelfObject() >> g.ObjectInfo()).o.transform
                        >> g.InvertMatrix(),
                        (move >> g.ObjectInfo()).o.transform,
                    ),
                    g.MultiplyMatrices(
                        (choose >> g.ObjectInfo()).o.transform >> g.InvertMatrix(),
                        (g.SelfObject() >> g.ObjectInfo()).o.transform,
                    ),
                ),
            )
        with g.Frame(label="Store Flag") as _:
            inside = inside >> g.Reroute()
            geometry = (
                geometry
                >> g.StoreNamedAttribute.point.boolean(
                    selection=inside, name="squishy_volumes_has_goal", value=True
                )
                >> g.SetPosition(selection=inside, position=goal_position)
            )

        with g.Frame(label="Store Goal") as _:
            (
                geometry
                >> g.StoreNamedAttribute.point.vector(
                    name="squishy_volumes_goal_position",
                    value=g.Position()
                    >> g.TransformPoint(
                        transform=(g.SelfObject() >> g.ObjectInfo()).o.transform
                    ),
                )
                >> tree.outputs.geometry()
            )

    tree.tree.is_modifier = True
    return tree.tree.name


datablocks: set[bpy.types.ID] = set()
for name in []:
    material = bpy.data.materials[name]
    material.asset_mark()
    datablocks.add(material)
for name in [
    generate_particles(),
    set_goals(),
]:
    node_group = bpy.data.node_groups[name]
    node_group.asset_mark()
    datablocks.add(node_group)

bpy.data.libraries.write(
    str(asset_file),
    datablocks,
)
