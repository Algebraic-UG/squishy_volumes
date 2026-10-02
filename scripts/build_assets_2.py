import glob
import uuid
import bpy
import sys
from pathlib import Path

from ntpath import split
from typing import Any, Literal, Set, TYPE_CHECKING

from nodebpy import SimpleOptions, TreeBuilder, arrange, SugiyamaOptions
from nodebpy import geometry as g
from nodebpy.types import (
    InputGeometry,
    InputString,
    InputMenu,
    InputFloat,
    InputBoolean,
    InputInteger,
    InputVector,
    InputColor,
    InputRotation,
    InputMatrix,
    InputAny,
    InputBundle,
)
from nodebpy.nodes.geometry import CombineBundle
from nodebpy.builder import (
    InputInterfaceContext,
    CustomGeometryGroup,
    FloatSocket,
    IntegerSocket,
    VectorSocket,
    MatrixSocket,
    SocketAccessor,
    GeometrySocket,
    StringSocket,
    BooleanSocket,
)

scripts_dir = Path(sys.argv[0]).parent
repo_root = scripts_dir / ".."
asset_dir = repo_root / "python" / "src" / "squishy_volumes_extension" / "assets"
unique = uuid.uuid4().hex[:8]
asset_file = asset_dir / f"assets-{unique}.blend"

for stale_asset in glob.glob(pathname="assets-*.blend", root_dir=asset_dir):
    print(f"Removing stale asset: {stale_asset}")
    (asset_dir / stale_asset).unlink()


_DataType = Literal[
    "FLOAT",
    "INT",
    "BOOLEAN",
    "FLOAT_VECTOR",
    "FLOAT_COLOR",
    "QUATERNION",
    "FLOAT4X4",
]

# data_type → tree.inputs / tree.outputs factory name
_INTERFACE_METHOD = {
    "FLOAT": "float",
    "INT": "integer",
    "BOOLEAN": "boolean",
    "FLOAT_VECTOR": "vector",
    "FLOAT_COLOR": "color",
    "QUATERNION": "rotation",
    "FLOAT4X4": "matrix",
}


class StoreNamedAttributeIfRecord[T](CustomGeometryGroup):
    _name = "Store Named Attribute If Record"
    _data_type: _DataType = "FLOAT"
    _color_tag = "ATTRIBUTE"

    class _Inputs[S](SocketAccessor):
        geometry: GeometrySocket
        """Geometry"""
        name: StringSocket
        """Name"""
        value: S
        """Value"""

    class _Outputs(SocketAccessor):
        geometry: GeometrySocket
        """Geometry"""

    if TYPE_CHECKING:

        @property
        def i(self) -> _Inputs[T]: ...
        @property
        def o(self) -> _Outputs: ...

    def __init__(
        self,
        geometry: InputGeometry = None,
        name: InputString = "",
        value: InputAny = None,
        *,
        data_type: _DataType = "FLOAT",
    ):
        # Set before super().__init__(): it picks and builds the inner tree.
        self._data_type = data_type
        super().__init__(Geometry=geometry, Name=name, Value=value)

    @property
    def data_type(self) -> _DataType:
        """Read-only: the group's socket types are fixed once built, so a
        different data type needs a new node."""
        return self._data_type

    def _group_name(self) -> str:
        return f"{self._name} ({self.data_type})"

    def _build_group(self, tree: TreeBuilder[Any]) -> None:
        geometry = tree.inputs.geometry("Geometry")
        name = tree.inputs.string("Name")
        value = getattr(tree.inputs, _INTERFACE_METHOD[self.data_type])("Value")

        (
            (
                geometry
                >> g.GetGeometryBundle()
                >> g.GetBundleItem.bundle(path="Record")
            ).o.item
            >> g.GetBundleItem.boolean(path=name)
        ).o.item.switch.geometry(
            geometry,
            g.StoreNamedAttribute(
                geometry=geometry,
                data_type=self.data_type,
                name=g.JoinStrings(["squishy_volumes", name], "_").o.string,
                value=value,
            ),
        ) >> tree.outputs.geometry("Geometry")

    @classmethod
    def float(
        cls,
        geometry: InputGeometry = None,
        name: InputString = "",
        value: InputFloat = 0.0,
    ) -> "StoreNamedAttributeIfRecord[FloatSocket]":
        return StoreNamedAttributeIfRecord(geometry, name, value, data_type="FLOAT")

    @classmethod
    def integer(
        cls,
        geometry: InputGeometry = None,
        name: InputString = "",
        value: InputInteger = 0,
    ) -> "StoreNamedAttributeIfRecord[IntegerSocket]":
        return StoreNamedAttributeIfRecord(geometry, name, value, data_type="INT")

    @classmethod
    def vector(
        cls,
        geometry: InputGeometry = None,
        name: InputString = "",
        value: InputVector = (0.0, 0.0, 0.0),
    ) -> "StoreNamedAttributeIfRecord[VectorSocket]":
        return StoreNamedAttributeIfRecord(
            geometry, name, value, data_type="FLOAT_VECTOR"
        )

    @classmethod
    def matrix(
        cls,
        geometry: InputGeometry = None,
        name: InputString = "",
        value: InputMatrix = None,
    ) -> "StoreNamedAttributeIfRecord[MatrixSocket]":
        return StoreNamedAttributeIfRecord(geometry, name, value, data_type="FLOAT4X4")


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
        super().__init__(
            **{
                "Geometry": geometry,
                "Position": position,
                "Seed": seed,
            }
        )

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


sna = g.StoreNamedAttribute.point


RECORDED = [
    ("Flags", "flags", sna.integer),
    ("Collider Bits", "collider_bits", sna.integer),
    ("Transform", "transform", sna.matrix),
    ("Size", "size", sna.float),
    ("Density", "density", sna.float),
    ("Young's Modulus", "youngs_modulus", sna.float),
    ("Poisson's Ratio", "poissons_ratio", sna.float),
    ("Initial Position", "initial_position", sna.vector),
    ("Initial Velocity", "initial_velocity", sna.vector),
    ("Viscosity Dynamic", "viscosity_dynamic", sna.float),
    ("Viscosity Bulk", "viscosity_bulk", sna.float),
    ("Exponent", "exponent", sna.integer),
    ("Bulk Modulus", "bulk_modulus", sna.float),
    ("Sand Alpha", "sand_alpha_value", sna.float),
    ("Goal Position", "goal_position", sna.vector),
]


class SetFlag(CustomGeometryGroup):
    _name = "Set Flag"
    _color_tag = "ATTRIBUTE"

    def __init__(
        self,
        geometry: InputGeometry = ...,
        flag: InputMenu = ...,
        value: InputBoolean = False,
    ):
        super().__init__(
            **{
                "Geometry": geometry,
                "Flag": flag,
                "Value": value,
            }
        )

    def _build_group(self, tree):
        geometry = tree.inputs.geometry("Geometry")
        flag = tree.inputs.menu("Flag")
        value = tree.inputs.boolean("Value")

        bit = g.BitMath.shift(
            value.switch.integer(false=0, true=1),
            g.MenuSwitch.integer(
                menu=flag,
                items={
                    "IS_SOLID": 0,
                    "IS_FLUID": 1,
                    "USE_VISCOSITY": 2,
                    "USE_SAND_ALPHA": 3,
                    "HAS_GOAL": 4,
                    "TOMBSTONED": 5,
                    "FAILED": 6,
                },
            ),
        )
        (
            geometry
            >> StoreNamedAttributeIfRecord.integer(
                name="flags",
                value=g.NamedAttribute.integer(name="squishy_volumes_flags")
                >> g.BitMath.l_and(b=g.BitMath.l_not(bit))
                >> g.BitMath.l_or(b=bit),
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
            (
                (geometry >> g.GetGeometryBundle()).o.bundle
                >> g.GetBundleItem.bundle(
                    path="Parameter",
                )
            ).o.item
            >> g.SeparateBundle()
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
            geometry = geometry >> StoreNamedAttributeIfRecord.matrix(
                name="transform",
                value=g.CombineTransform(
                    g.TransformPoint(g.Position(), info.o.transform),
                    info.o.rotation,
                    info.o.scale,
                ),
            )

        with g.Frame("Initial Velocity") as _:
            geometry = geometry >> StoreNamedAttributeIfRecord.vector(
                name="initial_velocity",
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
                >> StoreNamedAttributeIfRecord.float(name="size", value=size)
                >> StoreNamedAttributeIfRecord.float(name="density", value=density)
                >> StoreNamedAttributeIfRecord.float(
                    name="viscosity_dynamic", value=viscosity_dynamic
                )
                >> StoreNamedAttributeIfRecord.float(
                    name="viscosity_bulk", value=viscosity_bulk
                )
            )

        with g.Frame("Solid Parameters") as _:
            solid = (
                geometry
                >> StoreNamedAttributeIfRecord.float(
                    name="youngs_modulus",
                    value=youngs_modulus,
                )
                >> StoreNamedAttributeIfRecord.float(
                    name="poissons_ratio",
                    value=poissons_ratio,
                )
                >> StoreNamedAttributeIfRecord.float(
                    name="sand_alpha_value",
                    value=sand_alpha_value,
                )
            )

        with g.Frame("Fluid Parameters") as _:
            fluid = (
                geometry
                >> StoreNamedAttributeIfRecord.float(
                    name="bulk_modulus",
                    value=bulk_modulus,
                )
                >> StoreNamedAttributeIfRecord.integer(
                    name="exponent",
                    value=exponent,
                )
            )

        geometry = g.IndexSwitch.geometry(material_type, [solid, fluid])

        with g.Frame("Flags") as _:
            geometry = (
                geometry
                >> SetFlag(
                    flag="IS_SOLID", value=g.Compare.integer.equal(0, material_type)
                )
                >> SetFlag(
                    flag="IS_FLUID", value=g.Compare.integer.equal(1, material_type)
                )
                >> SetFlag(flag="USE_VISCOSITY", value=viscosity)
                >> SetFlag(flag="USE_SAND_ALPHA", value=sand_alpha)
            )

        geometry >> tree.outputs.geometry()


materials = []
node_groups = []


def make_bundle(inputs) -> CombineBundle:
    return g.CombineBundle({i.name: i for i in inputs})


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

    with tree.inputs.panel("Record"):
        record = tree.inputs.boolean(
            "Record",
            is_panel_toggle=True,
            structure_type="SINGLE",
            default_value=True,
        )
        record_bundle = make_bundle(
            tree.inputs.boolean(
                attribute,
                structure_type="SINGLE",
                default_value=True,
            )
            for _label, attribute, _node in RECORDED
        )

    parameters = dict()
    parameters["Size"] = spacing

    with tree.inputs.panel("Parameters"):
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
            sand_alpha = tree.inputs.boolean(name="Sand Alpha", is_panel_toggle=True)
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
        record.switch.geometry(
            false=points,
            true=points
            >> g.SetGeometryBundle(
                bundle=g.CombineBundle(
                    {"Record": record_bundle, "Parameter": g.CombineBundle(parameters)}
                )
            )
            >> Record(),
        )
        >> tree.outputs.geometry()
    )

tree.tree.is_modifier = True
node_groups.append(tree.tree.name)

datablocks: set[bpy.types.ID] = set()
for name in materials:
    material = bpy.data.materials[name]
    material.asset_mark()
    datablocks.add(material)
for name in node_groups:
    node_group = bpy.data.node_groups[name]
    node_group.asset_mark()
    datablocks.add(node_group)

bpy.data.libraries.write(
    str(asset_file),
    datablocks,
)
