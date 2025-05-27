from fastapi import FastAPI

from core.task import Task
from core.graph import TaskGraph

app = FastAPI()
TG = TaskGraph()


@app.get("/")
async def root():
    return {"message": "Hello World"}


@app.get("/health")
async def health():
    return {"status": "healthy", "service": "meshwork-backend"}


@app.post("/v0/task/add/")
async def add_task(task: Task):
    TG.add_task(task)
    return {"message": f"Task {task.id} added"}


@app.get("/v0/task/{task_id}")
async def get_task(task_id: str) -> Task:
    return TG.get_task(task_id)


@app.get("/v0/task/get_all")
async def get_all_tasks() -> list[Task]:
    return TG.get_all_tasks()


@app.delete("/v0/task/{task_id}")
async def delete_task(task_id: str):
    TG.delete_task(task_id)
    return {"message": f"Task {task_id} deleted"}


@app.put("/v0/task/{task_id}")
async def edit_task(task_id: str, new_task: Task):
    TG.edit_task(task_id, new_task)
    return {"message": f"Task {task_id} edited"}
