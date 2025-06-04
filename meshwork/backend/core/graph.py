import json
import datetime
import logging
import time
import uuid

import schedule
import networkx as nx
from networkx.readwrite import json_graph
from networkx import DiGraph
from pydantic import BaseModel, Field

from core.task import Task, Status

logger = logging.getLogger(__name__)


class TaskGraph(BaseModel):
    """A directed graph representing tasks and their dependencies."""

    id: str = Field(default=uuid.uuid4().hex)
    graph: DiGraph = Field(default_factory=DiGraph)
    name: str = Field(default_factory=str)
    backup_interval: int = Field(default=10)
    """Interval in seconds for automatic backup."""
    backup_folder: str = Field(default="backup")

    class Config:
        arbitrary_types_allowed = True

    def __init__(self, **data):
        super().__init__(**data)
        if "backup_interval" in data:
            self.backup_interval = data["backup_interval"]
        schedule.every(self.backup_interval).seconds.do(self.backup_json)
        self.id = uuid.uuid4().hex

    def load_backup(self):
        # TODO: Implement loading the file with the most recent timestamp in filename
        raise NotImplementedError("Backup loading is not implemented yet.")

    def add_task(self, task: Task):
        """Add a task to the graph."""
        self.graph.add_node(task.id, task=task)
        logger.info(f"Added task {task.id} to the graph")
        for d in task.depends_on:
            self.graph.add_edge(d, task.id)
            logger.info(f"Added dependency {d} -> {task.id}")

        self.set_blocked_tasks()

    def get_task(self, task_id: str):
        """Get a task from the graph."""
        return self.graph.nodes[task_id]["task"]

    def get_all_tasks(self):
        """Get all tasks from the graph."""
        if not self.graph.nodes:
            return []
        return [self.graph.nodes[node]["task"] for node in self.graph.nodes]

    def edit_task(self, task_id: str, fields: dict):
        # TODO: take only the necessary fields from new_task
        """Edit a task in the graph."""
        for key, value in fields.items():
            self.graph.nodes[task_id]["task"].key = value
        self.set_blocked_tasks()

    def connect_nodes(self, node_dependency: str, node_dependee: str):
        """Connect two nodes in the graph."""
        self.graph.add_edge(node_dependency, node_dependee)
        self.graph.nodes[node_dependee]["task"].depends_on.append(node_dependency)
        logger.info(f"Connected nodes {node_dependency} and {node_dependee}")

        self.set_blocked_tasks()


    def disconnect_nodes(self, node_dependency: str, node_dependee: str):
        """Disconnect two nodes in the graph."""
        self.graph.remove_edge(node_dependency, node_dependee)
        self.graph.nodes[node_dependee]["task"].depends_on.remove(node_dependency)
        logger.info(f"Disconnected nodes {node_dependency} and {node_dependee}")

        self.set_blocked_tasks()


    def set_blocked_tasks(self):
        """
        Iterate over graph and set tasks as blocked if their dependencies are not DONE.
        """
        for node in self.graph.nodes:
            task = self.graph.nodes[node]["task"]
            if not all(
                self.get_task(dep).status == Status.DONE for dep in task.depends_on
            ):
                task.status = Status.BLOCKED
            else:
                if task.status == Status.BLOCKED:
                    task.status = Status.TODO

    def delete_task(self, task_id: str):
        """Delete a task from the graph."""
        self.graph.remove_node(task_id)
        # iterate over all nodes in the graph
        for node in self.graph.nodes:
            task = self.graph.nodes[node]["task"]
            if task_id in task.depends_on:
                task.depends_on.remove(task_id)
        self.set_blocked_tasks()

    def visualize(self):
        import matplotlib.pyplot as plt
        pos = nx.kamada_kawai_layout(self.graph)
        nx.draw(self.graph, pos=pos, with_labels=True, arrows=True)
        plt.show()  # This line is crucial to display the figure

    def backup_json(self):
        # Convert graph data to a serializable format
        json_data = json_graph.node_link_data(self.graph)

        # Convert all Task objects to dicts and handle Status enum
        for node in json_data["nodes"]:
            if "task" in node:
                task_dict = node["task"].dict()  # Convert Pydantic model to dict
                task_dict["status"] = task_dict["status"].value  # Convert enum to value
                node["task"] = task_dict

        filename = f"{datetime.datetime.now().strftime('%Y%m%d%H%M%S')}_{self.name}_backup.json"
        with open(filename, "w") as f:
            json.dump(json_data, f, indent=4)
        logger.info(f"Backup created: {filename}")


# Example usage
if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)  # <-- Enable logging
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

    while True:
        schedule.run_pending()
        time.sleep(1)
