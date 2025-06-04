import logging

from pydantic import BaseModel
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from core.task import Task, Status
from core.user import User
from core.graph import TaskGraph

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

def setup_examples() -> tuple[TaskGraph, User]:
    """Initialize example tasks and users"""
    example_graph = TaskGraph()
    example_graph.id = "abc123"
    user = User(id=123, name="John Doe", email="john@example.com", graphs=[example_graph.id])
    task_1 = Task(name="Create pretty task nodes", description="Learn UI and do it!", status=Status.TODO, tags=["UI", "UX"], users=["Andrew"])
    logger.info(f"Created task {task_1.id}")
    task_2 = Task(name="Add CI/CD support", description="Things to test your Svelte", status=Status.TODO, tags=["CI/CD"], users=["Andrew"])
    logger.info(f"Created task {task_2.id}")
    task_3 = Task(name="Release v0.1", description="Hooray!", status=Status.TODO, tags=["v0.1"], depends_on=[task_1.id, task_2.id])
    logger.info(f"Created task {task_3.id}")
    example_graph.add_task(task_1)
    example_graph.add_task(task_2)
    example_graph.add_task(task_3)
    return example_graph, user

app = FastAPI()
TG, user = setup_examples()
logger.info(f"Initialized graph {TG.id}")
task_graphs = {
    TG.id: TG
}

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

class ConnectNodesRequest(BaseModel):
    node_dependency: str
    node_dependee: str

@app.get("/")
async def root():
    return {"message": "Hello World"}


@app.get("/health")
async def health():
    return {"status": "healthy", "service": "meshwork-backend"}


@app.post("/v0/create_graph")
async def create_graph():
    new_graph = TaskGraph()
    task_graphs[new_graph.id] = new_graph
    return {"ID": new_graph.id}

@app.post("/v0/{graph_id}/task/add/")
async def add_task(graph_id: str, task: Task):
    task_graphs[graph_id].add_task(task)
    return {"message": f"Task {task.id} added"}


@app.get("/v0/{graph_id}/all_tasks")
async def get_all_tasks(graph_id: str) -> list[Task]:
    return task_graphs[graph_id].get_all_tasks()

@app.get("/v0/{graph_id}/task/{task_id}")
async def get_task(graph_id: str, task_id: str) -> Task:
    return task_graphs[graph_id].get_task(task_id)


@app.delete("/v0/{graph_id}/task/{task_id}")
async def delete_task(graph_id: str, task_id: str):
    task_graphs[graph_id].delete_task(task_id)
    return {"message": f"Task {task_id} deleted"}


@app.put("/v0/{graph_id}/task/{task_id}")
async def edit_task(graph_id: str, task_id: str, new_task: Task):
    task_graphs[graph_id].edit_task(task_id, new_task)
    return {"message": f"Task {task_id} edited"}

@app.post("/v0/{graph_id}/connect_nodes/")
async def connect_nodes(graph_id: str, request: ConnectNodesRequest):
    task_graphs[graph_id].connect_nodes(request.node_dependency, request.node_dependee)
    return {"message": f"Nodes {request.node_dependency} and {request.node_dependee} connected"}

@app.post("/v0/{graph_id}/disconnect_nodes/")
async def disconnect_nodes(graph_id: str, request: ConnectNodesRequest):
    task_graphs[graph_id].disconnect_nodes(request.node_dependency, request.node_dependee)
    return {"message": f"Nodes {request.node_dependency} and {request.node_dependee} disconnected"}
