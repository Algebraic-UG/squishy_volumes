import sys
from pathlib import Path

import bpy
from bpy.types import GeometryNodeTree
from nodebpy import TreeBuilder
from nodebpy import geometry as g
from nodebpy.builder import CustomGeometryGroup, MatrixSocket
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

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry()
        sep = geometry >> g.GetGeometryBundle() >> g.SeparateBundle()

        spacing = sep.items.float("Spacing").output
        random = sep.items.float("Random").output

        with g.Frame("Calculate Extents"):
            bb = geometry >> g.BoundingBox()
            extents = bb.o.max - bb.o.min

        with g.Frame("Point Count, Total and Along XYZ"):
            xyz = extents.scale(1 / spacing)

            # we can iterate a VectorSocket directly, uses SeparateXYZ
            x, y, z = [c.to_integer("FLOOR").max(1) for c in xyz]

            xyz = g.CombineXYZ(x, y, z)

            total = x * y * z

        with g.Frame("Normalized Position From Index"):
            i = g.Index().o.index
            normalized_position = g.CombineXYZ(
                i // (y * z),
                i % (y * z) // z,
                i % z,
            )
        with g.Frame("Add Radomness"):
            normalized_position = normalized_position.o.vector + g.RandomValue.vector(
                min=-random,
                max=random,
            )

        with g.Frame("Denormalize Position"):
            redivided = extents / xyz

            position = g.VectorMath.multiply_add(
                redivided,
                normalized_position,
                g.VectorMath.multiply_add(
                    redivided,
                    0.5,
                    bb.o.min,
                ),
            )

        with g.Frame("Generate Points"):
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
        position: InputVector = (0.0, 0.0, 0.0),
        seed: InputInteger = 0,
    ):
        super().__init__(Geometry=geometry, Position=position, Seed=seed)

    def _build_group(self, tree):
        geometry = tree.inputs.geometry("Geometry")
        bb = geometry >> g.BoundingBox()
        extents = bb.o.max - bb.o.min

        dir = g.RandomValue.vector(
            min=-1,  # vector inputs accept single-values (they re-use)
            max=1.0,
            seed=tree.inputs.integer("Seed"),
        )
        raycast = g.Raycast(
            target_geometry=geometry,
            source_position=tree.inputs.vector("Position"),
            ray_direction=dir,
            # .x/.y/.z auto-add a single SeparateXYZ() and re-use it
            ray_length=extents.x + extents.y + extents.z,
        )

        (
            raycast.o.is_hit & (raycast.o.hit_normal.dot(dir) > 0)
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
                selection=~(
                    IsInsideObject(geometry, position, 0)
                    & IsInsideObject(geometry, position, 1)
                )
            )
            >> tree.outputs.geometry()
        )


class Record(CustomGeometryGroup):
    _name = "Particle Parameters"

    def __init__(self):
        super().__init__()

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry()

        items = (geometry >> g.GetGeometryBundle() >> g.SeparateBundle()).items
        initial_velocity_linear = items.vector("Initial Velocity Linear").output
        initial_velocity_angular = items.vector("Initial Velocity Angular").output
        density = items.float("Density").output
        viscosity_dynamic = items.float("Viscosity Dynamic").output
        viscosity_bulk = items.float("Viscosity Bulk").output
        youngs_modulus = items.float("Young's Modulus").output
        poissons_ratio = items.float("Poissons's Ratio").output
        sand_alpha_value = items.float("Sand Alpha Value").output
        bulk_modulus = items.float("Bulk Modulus").output
        exponent = items.integer("Exponent").output
        material_type = items.integer("Type").output
        viscosity = items.boolean("Viscosity").output
        sand_alpha = items.boolean("Sand Alpha").output
        size = items.float("Size").output

        trans = g.SelfObject().o.self_object.matrix()
        pos = g.Position().o.position

        with g.Frame("Object Transform"):
            geometry = geometry >> g.StoreNamedAttribute.point.matrix(
                name="squishy_volumes_transform",
                value=g.CombineTransform(
                    pos.transform(trans),
                    trans.rotation,
                    trans.scale,
                ),
            )

        with g.Frame("Initial Velocity"):
            geometry = geometry >> g.StoreNamedAttribute.point.vector(
                name="squishy_volumes_velocity",
                value=initial_velocity_linear
                + initial_velocity_angular.cross(pos.transform_direction(trans)),
            )

        with g.Frame("Common Parameters"):
            geometry = (
                geometry
                >> g.StoreNamedAttribute.point.float(
                    name="squishy_volumes_size", value=size
                )
                >> g.StoreNamedAttribute.point.float(
                    name="squishy_volumes_density", value=density
                )
                >> g.StoreNamedAttribute.point.float(
                    name="squishy_volumes_viscosity_dynamic", value=viscosity_dynamic
                )
                >> g.StoreNamedAttribute.point.float(
                    name="squishy_volumes_viscosity_bulk", value=viscosity_bulk
                )
            )

        with g.Frame("Solid Parameters"):
            solid = (
                geometry
                >> g.StoreNamedAttribute.point.float(
                    name="squishy_volumes_youngs_modulus",
                    value=youngs_modulus,
                )
                >> g.StoreNamedAttribute.point.float(
                    name="squishy_volumes_poissons_ratio",
                    value=poissons_ratio,
                )
                >> g.StoreNamedAttribute.point.float(
                    name="squishy_volumes_sand_alpha_value",
                    value=sand_alpha_value,
                )
            )

        with g.Frame("Fluid Parameters"):
            fluid = (
                geometry
                >> g.StoreNamedAttribute.point.float(
                    name="squishy_volumes_bulk_modulus",
                    value=bulk_modulus,
                )
                >> g.StoreNamedAttribute.point.integer(
                    name="squishy_volumes_exponent",
                    value=exponent,
                )
            )

        geometry = g.IndexSwitch.geometry(material_type, [solid, fluid])

        with g.Frame("Flags"):
            geometry = (
                geometry
                >> g.StoreNamedAttribute.point.boolean(
                    name="squishy_volumes_is_solid",
                    value=material_type == 0,
                )
                >> g.StoreNamedAttribute.point.boolean(
                    name="squishy_volumes_is_fluid",
                    value=material_type == 1,
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

        parameters = {}
        parameters["Size"] = spacing

        with tree.inputs.panel("Initial Parameters"):
            start_frame = tree.inputs.integer("Start Frame", default_value=1)

            parameters["Density"] = tree.inputs.float("Density", default_value=1000.0)
            type_switch = tree.inputs.menu(name="Type") >> g.MenuSwitch.integer()
            is_solid = type_switch.items.new(0, "Solid").output
            is_fluid = type_switch.items.new(1, "Fluid").output

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

            with tree.inputs.panel(name="Viscosity"):
                viscosity = tree.inputs.boolean(name="Viscosity", is_panel_toggle=True)
                parameters["Viscosity"] = viscosity
                parameters["Viscosity Dynamic"] = viscosity.switch.float(
                    true=tree.inputs.float("Viscosity Dynamic", default_value=3.0)
                )
                parameters["Viscosity Bulk"] = viscosity.switch.float(
                    true=tree.inputs.float("Viscosity Bulk", default_value=3.0)
                )

            with tree.inputs.panel(name="Sand Alpha"):
                sand_alpha = tree.inputs.boolean(
                    name="Sand Alpha", is_panel_toggle=True
                )
                parameters["Sand Alpha"] = sand_alpha
                parameters["Sand Alpha Value"] = sand_alpha.switch.float(
                    true=tree.inputs.float("Sand Alpha Value", default_value=0.3)
                )

            with tree.inputs.panel(name="Initial Velocity"):
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

        with g.Frame(label="Detect Choosen Particles"):
            choose_geometry = choose.geometry("RELATIVE")
            position = g.Position()
            inside = IsInsideObject(choose_geometry, position, 0) & IsInsideObject(
                choose_geometry, position, 1
            )

        def self_matrix() -> MatrixSocket:
            return g.SelfObject().o.self_object.matrix()

        with g.Frame(label="Local -> Chooser -> Mover -> Back to Local"):
            goal_position = g.Position().o.position.transform(
                self_matrix().invert()
                @ move.matrix()
                @ choose.matrix().invert()
                @ self_matrix()
            )
        with g.Frame(label="Store Flag"):
            inside = inside >> g.Reroute()
            geometry = (
                geometry
                >> g.StoreNamedAttribute.point.boolean(
                    selection=inside, name="squishy_volumes_has_goal", value=True
                )
                >> g.SetPosition(selection=inside, position=goal_position)
            )

        with g.Frame(label="Store Goal"):
            (
                geometry
                >> g.StoreNamedAttribute.point.vector(
                    name="squishy_volumes_goal_position",
                    value=g.Position().o.position.transform(self_matrix()),
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
