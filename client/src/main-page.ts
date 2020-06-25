import * as d3 from 'd3';
import { LitElement, html, css, customElement, property, query } from 'lit-element';

import { Data, Node, Link } from './types';
import * as helpers from './helpers';
import forceLink from './forceLink';

@customElement('main-page')
export class MainPage extends LitElement {
    @query('#chart')
    chart!: d3.BaseType | SVGElement

    @query('#tooltip')
    tooltip!: d3.BaseType | SVGElement

    data!: Data;
    simulation!: d3.Simulation<d3.SimulationNodeDatum, undefined>;

    constructor() {
        super();
    }

    firstUpdated(_changedProperties: any) {
        console.log(`Loaded ${this.tagName}.`);
        this.refresh();
    }

    private async refresh() {
        const response = await fetch('/api/similarities');
        const json: any[] = await response.json();
        
        let nodes = [...new Set(json.map(l => l.source_handle).concat(json.map(l => l.target_handle)))].map(s => { return { id: s } });

        let links = [...new Set(json)].map(l => {
            return {
                source: l.source_handle,
                target: l.target_handle,
                strength: l.strength
            };
        });

        helpers.shuffle(nodes);
        helpers.shuffle(links);

        this.data = {
            nodes,
            links
        };

        this.buildChart();
    }

    private buildChart() {
        const vw = Math.max(document.documentElement.clientWidth || 0, window.innerWidth || 0);
        const vh = Math.max(document.documentElement.clientHeight || 0, window.innerHeight || 0);

        const width = vw - 20;
        const height = vh - 20;

        const minStrength = Math.min(...this.data.links.map(l => l.strength));
        const maxStrength = Math.max(...this.data.links.map(l => l.strength));
        const averageStrength = this.data.links.reduce((agg, v) => agg + v.strength, 0) / this.data.links.length;
        const standardDeviation = Math.sqrt(this.data.links.reduce((agg, v) => agg + Math.pow(v.strength - averageStrength, 2), 0) / this.data.links.length);

        const cutoff = averageStrength;

        console.log(`Average link strength: ${averageStrength}.`);
        console.log(`Link strength standard deviation: ${standardDeviation}.`);

        const nodeRadius = 5;

        const chart = d3.select(this.chart).attr('width', width).attr('height', height).attr("viewBox", [0, 0, width, height] as any);

        const background = chart.append('rect')
            .attr('width', '100%')
            .attr('height', '100%')
            .attr('fill', '#999');
            
        const svg = chart.append("g");
        
        // Add zoom to SVG.
        chart.call(d3.zoom()
            .extent([[0, 0], [width, height]])
            .scaleExtent([1, 20])
            .on("zoom", zoomed) as any);

        const tooltip = d3.select(this.tooltip)		
            .style("opacity", 0);

        const linkSelection = svg.append('g')
                .attr('stroke-width', 0.3)
            .selectAll('line')
            .data(this.data.links)
            .join('line');

        const nodeSelection = svg.append('g')
                .attr('fill', '#fff')
                .attr('stroke', '#000')
                .attr('stroke-width', 0.3)
            .selectAll('circle')
            .data(this.data.nodes)
            .join('circle')
                .attr('r', nodeRadius)
                .on('mouseover', mouseover)
                .on('mouseout', mouseout);
        
        const nodeTextSelection = svg.append('g')
                .attr('fill', 'black')
                .attr("font-family", "sans-serif")
                .attr('font-size', '1.4px')
                .attr('dominant-baseline', 'middle')
                .attr('text-anchor', 'middle')
            .selectAll('text')
            .data(this.data.nodes)
            .join('text');

        // Make simulation.
        this.simulation = d3.forceSimulation().alphaDecay(.01)
            .nodes(this.data.nodes)
            // Nodes pull each other together by their strength, but only if above some value
            .force('link', forceLink<Node, Link>().id(d => (d as any).id).links(this.data.links).distance(0).strength(l => (l as any).strength - cutoff).iterations(2))
            .force('collision', d3.forceCollide(nodeRadius + 2).strength(1))
            .force('charge', d3.forceManyBody().strength(-10))
            .force('center', d3.forceCenter(width / 2, height / 2));

        const linkColorInterpolater = d3.piecewise(d3.interpolateRgb.gamma(2.2), ["red", "green", "blue"]);

        // Indicate nodes.
        this.simulation.on('tick', () => {
            linkSelection
                .attr('stroke', l => {
                    if(l.strength > cutoff) {
                        let value = .5 + (l.strength - cutoff) / (maxStrength - cutoff);
                        return linkColorInterpolater(value);
                    } else {
                        let value = .5 + (l.strength - cutoff) / (cutoff - minStrength);
                        return linkColorInterpolater(value);
                    }
                })
                .attr('stroke-opacity', 0.1)
                .attr('x1', l => (l.source as any).x)
                .attr('y1', l => (l.source as any).y)
                .attr('x2', l => (l.target as any).x)
                .attr('y2', l => (l.target as any).y);

            nodeSelection
                .attr('cx', d => (d as any).x)
                .attr('cy', d => (d as any).y);

            nodeTextSelection
                .attr('x', d => (d as any).x)
                .attr('y', d => (d as any).y)
                .text(n => n.id);
        });

        // Helper functions.

        let self = this;

        function mouseover(d: Node) {
            let topLinks = self.data.links.filter(l => (l.source as any).id === d.id || (l.target as any).id === d.id).sort((a, b) => b.strength - a.strength).slice(0, 10).map(n => {
                let other = '';

                if((n.source as any).id === d.id)
                    other = (n.target as any).id as string;
                else
                    other = (n.source as any).id as string;

                return {
                    handle: other,
                    strength: n.strength
                };
            });

            tooltip.html('');

            tooltip.transition()		
                .duration(200)		
                .style("opacity", .95);

            tooltip.append('h2').text(d.id);

            tooltip
                .style("left", (d3.event.pageX + 20) + "px")	
                .style("top", (d3.event.pageY + 20) + "px");

            tooltip
                .selectAll('p')
                .data(topLinks)
                .join('p')
                    .text(o => `${o.handle} => ${(o.strength * 100.0).toPrecision(3)}`);
        }

        function mouseout(d: Node) {
            tooltip.transition()		
                .duration(500)		
                .style("opacity", 0);
        }

        function zoomed() {
            svg.attr("transform", d3.event.transform);
        }
    }

    render() {
        return html`
            <svg id='chart' style='margin: auto; display: block;'></svg>
            <div id="tooltip" style="position:absolute;"></div>
        `;
    }

    static get styles() {
        return css`
            #tooltip {
                position: absolute;
                text-align: center;
                padding: 5px;
                font: 12px sans-serif;
                background: #bbb;
                border: 0px;
                border-radius: 2px;
                pointer-events: none;
                z-index: 1000;
            }

            svg text {
                -webkit-user-select: none;
                   -moz-user-select: none;
                    -ms-user-select: none;
                        user-select: none;
            }

            svg text::selection {
                background: none;
            }
        `;
    }
}