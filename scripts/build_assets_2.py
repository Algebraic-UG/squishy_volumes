import sys
from pathlib import Path
from typing import TYPE_CHECKING, Any, Literal

import bpy
from bpy.types import GeometryNodeTree
from nodebpy import TreeBuilder
from nodebpy import geometry as g
from nodebpy.builder import (
    CustomGeometryGroup,
    FloatSocket,
    GeometrySocket,
    IntegerSocket,
    MatrixSocket,
    SocketAccessor,
    StringSocket,
    VectorSocket,
)
from nodebpy.nodes.geometry import CombineBundle
from nodebpy.types import (
    InputAny,
    InputBoolean,
    InputFloat,
    InputGeometry,
    InputInteger,
    InputMatrix,
    InputMenu,
    InputString,
    InputVector,
)

scripts_dir = Path(sys.argv[0]).parent
repo_root = scripts_dir / ".."
asset_dir = repo_root / "python" / "src" / "squishy_volumes_extension" / "assets"
asset_file = asset_dir / "assets.blend"

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

            def floor_and_max(c: FloatSocket) -> IntegerSocket:
                return (
                    c
                    >> g.FloatToInteger(rounding_mode="FLOOR")
                    # ... passes the chained value into this argument
                    >> g.IntegerMath.maximum(..., 1)
                ).o.value

            # we can iterate a VectorSocket directly, uses SeparateXYZ
            x, y, z = [floor_and_max(c) for c in xyz]

            xyz = g.CombineXYZ(x, y, z)

            total = x * y * z

        with g.Frame("Normalized Position From Index"):
            i = g.Index()
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
        super().__init__(Geometry=geometry, Flag=flag, Value=value)

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

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
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
            geometry = geometry >> StoreNamedAttributeIfRecord.matrix(
                name="transform",
                value=g.CombineTransform(
                    pos.transform(trans),
                    trans.rotation,
                    trans.scale,
                ),
            )

        with g.Frame("Initial Velocity"):
            geometry = geometry >> StoreNamedAttributeIfRecord.vector(
                name="initial_velocity",
                value=initial_velocity_linear
                + initial_velocity_angular.cross(pos.transform_direction(trans)),
            )

        with g.Frame("Common Parameters"):
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

        with g.Frame("Solid Parameters"):
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

        with g.Frame("Fluid Parameters"):
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

        with g.Frame("Flags"):
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
