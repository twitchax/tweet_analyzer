import { Force, SimulationNodeDatum, SimulationLinkDatum } from 'd3';

function index(d: SimulationNodeDatum) {
    return d.index;
}

function constant(x: any) {
    return function() {
        return x;
    };
}

function jiggle() {
    return (Math.random() - 0.5) * 1e-6;
}

function find(nodeById: any, nodeId: any) {
    var node = nodeById.get(nodeId);
    if (!node) throw new Error("node not found: " + nodeId);
    return node;
}

export default function<NodeDatum extends SimulationNodeDatum, LinkDatum extends SimulationLinkDatum<NodeDatum>>(links: SimulationLinkDatum<SimulationNodeDatum>[] = [] as SimulationLinkDatum<SimulationNodeDatum>[]) {
    var id: any = index,
        strength: any = defaultStrength,
        strengths: number[],
        distance: any = constant(30),
        distances: number[],
        nodes: SimulationNodeDatum[],
        count: number[],
        iterations = 1;

    if (links == null) links = [];

    function defaultStrength(link: any | SimulationLinkDatum<SimulationNodeDatum>) {
        return 1 / Math.min(count[link.source.index], count[link.target.index]);
    }

    function force(alpha: number) {
        for (var k = 0, n = links.length; k < iterations; ++k) {
            for (var i = 0, link, source, target, x, y, l, b; i < n; ++i) {
                link = links[i], source = link.source as any, target = link.target as any;

                x = target.x + target.vx - source.x - source.vx || jiggle();
                y = target.y + target.vy - source.y - source.vy || jiggle();

                //l = Math.sqrt(x * x + y * y);

                let deltaX = 0;
                let deltaY = 0;
                let k = alpha * strengths[i];

                if(strengths[i] >= 0) {
                    // The force is "positive", so be a spring.
                    // The force for a spring is `k * x`, so the acceleration, or dv/dt
                    // is proportional to `k * x`.
                    deltaX = 2 * k * x;
                    deltaY = 2 * k * y;
                } else {
                    // The force is "negative", so be a charged particle.
                    // The force for a charged particle is `k / r^2`, so the acceleration, or dv/dt
                    // is proportional to `k / r^2`.
                    // Also, make the negative k positive.
                    // Nevermind, make this more like a weird "inverse spring".
                    deltaX = (k / x);
                    deltaY = (k / y);
                }

                target.vx -= deltaX;
                target.vy -= deltaY;
                source.vx += deltaX;
                source.vy += deltaY;
            }
        }
    }

    function initialize() {
        if (!nodes) return;

        var i,
            n = nodes.length,
            m = links.length,
            nodeById = new Map(nodes.map((d, i) => [id(d, i, nodes), d])),
            link;

        for (i = 0, count = new Array(n); i < m; ++i) {
        link = links[i] as any, link.index = i;
        if (typeof link.source !== "object") link.source = find(nodeById, link.source);
        if (typeof link.target !== "object") link.target = find(nodeById, link.target);
        count[link.source.index] = (count[link.source.index] || 0) + 1;
        count[link.target.index] = (count[link.target.index] || 0) + 1;
        }

        strengths = new Array(m), initializeStrength();
        distances = new Array(m), initializeDistance();
    }

    function initializeStrength() {
        if (!nodes) return;

        for (var i = 0, n = links.length; i < n; ++i) {
        strengths[i] = +strength(links[i], i, links);
        }
    }

    function initializeDistance() {
        if (!nodes) return;

        for (var i = 0, n = links.length; i < n; ++i) {
        distances[i] = +distance(links[i], i, links);
        }
    }

    force.initialize = function(_: any) {
        nodes = _;
        initialize();
    };

    force.links = function(_: any) {
        return arguments.length ? (links = _, initialize(), force) : links;
    };

    force.id = function(_: any) {
        return arguments.length ? (id = _, force) : id;
    };

    force.iterations = function(_: any) {
        return arguments.length ? (iterations = +_, force) : iterations;
    };

    force.strength = function(_: any) {
        return arguments.length ? (strength = typeof _ === "function" ? _ : constant(+_), initializeStrength(), force) : strength;
    };

    force.distance = function(_: any) {
        return arguments.length ? (distance = typeof _ === "function" ? _ : constant(+_), initializeDistance(), force) : distance;
    };

    return force as d3.ForceLink<SimulationNodeDatum, SimulationLinkDatum<SimulationNodeDatum>>;
}