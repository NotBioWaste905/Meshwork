import logging

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from core.task import Task
from core.graph import TaskGraph

logging.basicConfig(level=logging.INFO)

app = FastAPI()
TG = TaskGraph()
logger = logging.getLogger(__name__)
logger.info(f"Initialized graph {TG.id}")
task_graphs = {
    TG.id: TG
}

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3000"],  # React's default port
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

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
