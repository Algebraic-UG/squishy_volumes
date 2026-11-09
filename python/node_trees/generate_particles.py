# SPDX-License-Identifier: AGPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

from typing import Any, ClassVar

from bpy.types import GeometryNodeTree
from nodebpy import TreeBuilder
from nodebpy import geometry as g
from nodebpy.builder import CustomGeometryGroup

from ._shared import names
from ._shared.inside import is_inside_object


class GenerateGrid(CustomGeometryGroup):
    _name = "Generate Grid"

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
                g.VectorMath.multiply_add(redivided, 0.5, bb.o.min),
            )

        with g.Frame("Generate Points"):
            (
                g.Points(total, position)
                >> g.PointsToVertices()
                >> tree.outputs.geometry()
            )


class SampleParticles(CustomGeometryGroup):
    _name = "Sample Particles"

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry()
        (
            geometry
            >> GenerateGrid()
            >> g.DeleteGeometry(selection=~is_inside_object(geometry, g.Position()))
            >> tree.outputs.geometry()
        )


class ParticleParameters(CustomGeometryGroup):
    _name = "Particle Parameters"

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
                name=names.inputs.TRANSFORM,
                value=g.CombineTransform(
                    pos.transform(trans),
                    trans.rotation,
                    trans.scale,
                ),
            )

        with g.Frame("Initial Velocity"):
            geometry = geometry >> g.StoreNamedAttribute.point.vector(
                name=names.inputs.VELOCITY,
                value=initial_velocity_linear
                + initial_velocity_angular.cross(pos.transform_direction(trans)),
            )

        with g.Frame("Common Parameters"):
            geometry = (
                geometry
                >> g.StoreNamedAttribute.point.float(name=names.inputs.SIZE, value=size)
                >> g.StoreNamedAttribute.point.float(
                    name=names.inputs.DENSITY, value=density
                )
                >> g.StoreNamedAttribute.point.float(
                    name=names.inputs.VISCOSITY_DYNAMIC, value=viscosity_dynamic
                )
                >> g.StoreNamedAttribute.point.float(
                    name=names.inputs.VISCOSITY_BULK, value=viscosity_bulk
                )
            )

        with g.Frame("Solid Parameters"):
            solid = (
                geometry
                >> g.StoreNamedAttribute.point.float(
                    name=names.inputs.YOUNGS_MODULUS,
                    value=youngs_modulus,
                )
                >> g.StoreNamedAttribute.point.float(
                    name=names.inputs.POISSONS_RATIO,
                    value=poissons_ratio,
                )
                >> g.StoreNamedAttribute.point.float(
                    name=names.inputs.SAND_ALPHA,
                    value=sand_alpha_value,
                )
            )

        with g.Frame("Fluid Parameters"):
            fluid = (
                geometry
                >> g.StoreNamedAttribute.point.float(
                    name=names.inputs.BULK_MODULUS,
                    value=bulk_modulus,
                )
                >> g.StoreNamedAttribute.point.integer(
                    name=names.inputs.EXPONENT,
                    value=exponent,
                )
            )

        geometry = g.IndexSwitch.geometry(material_type, [solid, fluid])

        with g.Frame("Flags"):
            geometry = (
                geometry
                >> g.StoreNamedAttribute.point.boolean(
                    name=names.inputs.IS_SOLID,
                    value=material_type == 0,
                )
                >> g.StoreNamedAttribute.point.boolean(
                    name=names.inputs.IS_FLUID,
                    value=material_type == 1,
                )
                >> g.StoreNamedAttribute.point.boolean(
                    name=names.inputs.USE_VISCOSITY,
                    value=viscosity,
                )
                >> g.StoreNamedAttribute.point.boolean(
                    name=names.inputs.USE_SAND_ALPHA,
                    value=sand_alpha,
                )
            )

        geometry >> tree.outputs.geometry()


class GenerateParticles(CustomGeometryGroup):
    _name = "Squishy Volumes Generate Particles"
    _tree_properties: ClassVar[dict[str, Any]] = {"is_modifier": True}

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
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
                    true=tree.inputs.vector("Initial Velocity Linear")
                )
                parameters["Initial Velocity Angular"] = initial_velocity.switch.vector(
                    true=tree.inputs.vector("Initial Velocity Angular")
                )

        (
            (
                g.SceneTime().o.frame >> g.Compare.integer.equal(b=start_frame)
            ).o.result.switch.geometry(
                false=points,
                true=points
                >> g.SetGeometryBundle(bundle=g.CombineBundle(parameters))
                >> ParticleParameters(),
            )
            >> tree.outputs.geometry()
        )


ASSET = GenerateParticles
