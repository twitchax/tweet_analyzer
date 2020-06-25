import { SimulationNodeDatum, SimulationLinkDatum } from 'd3';

export type Data = {
    nodes: Node[];
    links: Link[];
}

export type Node = {
    id: string;
} & SimulationNodeDatum;

export type Link = {
    strength: number;
} & SimulationLinkDatum<Node>;