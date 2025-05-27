import networkx as nx
import matplotlib.pyplot as plt
from networkx import DiGraph
from pydantic import BaseModel, Field

from task import Task


class TaskGraph(BaseModel):
    graph: DiGraph = Field(default_factory=DiGraph)
    name: str = Field(default_factory=str)

    class Config:
        arbitrary_types_allowed = True

    def add_task(self, task: Task):
        self.graph.add_node(task.id, task=task)
        for d in task.depends_on:
            self.graph.add_edge(d, task.id)

    def visualize(self):
        # pos = nx.nx_agraph.pygraphviz_layout(self.graph)
        pos = nx.kamada_kawai_layout(self.graph)

        nx.draw(
            self.graph,
            pos,
            with_labels=True,
            node_color="lightblue",
            node_size=500,
            font_size=8,
            arrows=True,
        )
        # edge_labels = nx.get_edge_attributes(self.graph, "utterances")
        node_labels = nx.get_node_attributes(self.graph, "task")
        nx.draw_networkx_labels(self.graph, pos, labels=node_labels, font_size=10)
        plt.title(self.name)
        plt.axis("off")
        plt.show()


if __name__ == "__main__":
    TG = TaskGraph(name="MyTaskGraph")
    TG.add_task(
        Task(
            id="123", name="My subtask task", description="Blocking task", depends_on=[]
        )
    )
    TG.add_task(
        Task(
            id="321",
            name="My main task",
            description="Very important task",
            depends_on=["123"],
        )
    )
    TG.visualize()
