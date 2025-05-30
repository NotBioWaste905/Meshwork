import React, { useState, useEffect } from 'react';
import styled from 'styled-components';

const Overlay = styled.div`
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
`;

const Modal = styled.div`
  background: white;
  border-radius: 8px;
  padding: 24px;
  width: 500px;
  max-width: 90vw;
  max-height: 90vh;
  overflow-y: auto;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
`;

const Header = styled.div`
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
`;

const Title = styled.h2`
  margin: 0;
  color: #333;
  font-size: 18px;
`;

const CloseButton = styled.button`
  background: none;
  border: none;
  font-size: 24px;
  cursor: pointer;
  color: #666;
  padding: 0;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  
  &:hover {
    background: #f0f0f0;
  }
`;

const Form = styled.form`
  display: flex;
  flex-direction: column;
  gap: 16px;
`;

const FormGroup = styled.div`
  display: flex;
  flex-direction: column;
`;

const Label = styled.label`
  font-weight: 500;
  margin-bottom: 4px;
  color: #333;
  font-size: 14px;
`;

const Input = styled.input`
  padding: 8px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
  
  &:focus {
    outline: none;
    border-color: #007bff;
    box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
  }
`;

const Textarea = styled.textarea`
  padding: 8px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
  resize: vertical;
  min-height: 80px;
  font-family: inherit;
  
  &:focus {
    outline: none;
    border-color: #007bff;
    box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
  }
`;

const Select = styled.select`
  padding: 8px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
  background: white;
  
  &:focus {
    outline: none;
    border-color: #007bff;
    box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
  }
`;

const TagInput = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 6px;
  border: 1px solid #ddd;
  border-radius: 4px;
  min-height: 36px;
  
  &:focus-within {
    border-color: #007bff;
    box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
  }
`;

const Tag = styled.span`
  background: #e9ecef;
  color: #495057;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 4px;
`;

const TagRemoveButton = styled.button`
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  font-size: 12px;
  padding: 0;
  width: 14px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  
  &:hover {
    background: rgba(0, 0, 0, 0.1);
  }
`;

const TagInputField = styled.input`
  border: none;
  outline: none;
  flex: 1;
  min-width: 60px;
  font-size: 12px;
  padding: 2px;
`;

const ButtonGroup = styled.div`
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  margin-top: 20px;
`;

const Button = styled.button`
  padding: 8px 16px;
  border-radius: 4px;
  font-size: 14px;
  cursor: pointer;
  border: 1px solid;
  transition: all 0.2s ease;
  
  ${props => props.variant === 'primary' ? `
    background: #007bff;
    border-color: #007bff;
    color: white;
    
    &:hover {
      background: #0056b3;
      border-color: #0056b3;
    }
  ` : `
    background: white;
    border-color: #6c757d;
    color: #6c757d;
    
    &:hover {
      background: #6c757d;
      color: white;
    }
  `}
`;

const statusOptions = [
  { value: 0, label: 'TODO' },
  { value: 1, label: 'IN PROGRESS' },
  { value: 2, label: 'DONE' },
  { value: 3, label: 'REVIEW' },
  { value: 4, label: 'BLOCKED' }
];

const TaskEditor = ({ task, isOpen, onClose, onSave, allTasks = [] }) => {
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    status: 0,
    users: [],
    tags: [],
    depends_on: []
  });
  const [tagInput, setTagInput] = useState('');
  const [userInput, setUserInput] = useState('');

  useEffect(() => {
    if (task) {
      setFormData({
        name: task.name || '',
        description: task.description || '',
        status: task.status !== undefined ? task.status : 0,
        users: task.users || [],
        tags: task.tags || [],
        depends_on: task.depends_on || []
      });
    } else {
      setFormData({
        name: '',
        description: '',
        status: 0,
        users: [],
        tags: [],
        depends_on: []
      });
    }
  }, [task]);

  const handleInputChange = (field, value) => {
    setFormData(prev => ({
      ...prev,
      [field]: value
    }));
  };

  const handleTagKeyPress = (e) => {
    if (e.key === 'Enter' && tagInput.trim()) {
      e.preventDefault();
      if (!formData.tags.includes(tagInput.trim())) {
        handleInputChange('tags', [...formData.tags, tagInput.trim()]);
      }
      setTagInput('');
    }
  };

  const removeTag = (tagToRemove) => {
    handleInputChange('tags', formData.tags.filter(tag => tag !== tagToRemove));
  };

  const handleUserKeyPress = (e) => {
    if (e.key === 'Enter' && userInput.trim()) {
      e.preventDefault();
      if (!formData.users.includes(userInput.trim())) {
        handleInputChange('users', [...formData.users, userInput.trim()]);
      }
      setUserInput('');
    }
  };

  const removeUser = (userToRemove) => {
    handleInputChange('users', formData.users.filter(user => user !== userToRemove));
  };

  const handleSubmit = (e) => {
    e.preventDefault();
    onSave(formData);
  };

  const handleOverlayClick = (e) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <Overlay onClick={handleOverlayClick}>
      <Modal>
        <Header>
          <Title>{task ? 'Edit Task' : 'Create New Task'}</Title>
          <CloseButton onClick={onClose}>×</CloseButton>
        </Header>
        
        <Form onSubmit={handleSubmit}>
          <FormGroup>
            <Label>Task Name *</Label>
            <Input
              type="text"
              value={formData.name}
              onChange={(e) => handleInputChange('name', e.target.value)}
              placeholder="Enter task name"
              required
            />
          </FormGroup>

          <FormGroup>
            <Label>Description</Label>
            <Textarea
              value={formData.description}
              onChange={(e) => handleInputChange('description', e.target.value)}
              placeholder="Enter task description"
            />
          </FormGroup>

          <FormGroup>
            <Label>Status</Label>
            <Select
              value={formData.status}
              onChange={(e) => handleInputChange('status', parseInt(e.target.value))}
            >
              {statusOptions.map(option => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </Select>
          </FormGroup>

          <FormGroup>
            <Label>Assigned Users</Label>
            <TagInput>
              {formData.users.map((user, index) => (
                <Tag key={index}>
                  {user}
                  <TagRemoveButton onClick={() => removeUser(user)}>
                    ×
                  </TagRemoveButton>
                </Tag>
              ))}
              <TagInputField
                value={userInput}
                onChange={(e) => setUserInput(e.target.value)}
                onKeyPress={handleUserKeyPress}
                placeholder="Add user and press Enter"
              />
            </TagInput>
          </FormGroup>

          <FormGroup>
            <Label>Tags</Label>
            <TagInput>
              {formData.tags.map((tag, index) => (
                <Tag key={index}>
                  {tag}
                  <TagRemoveButton onClick={() => removeTag(tag)}>
                    ×
                  </TagRemoveButton>
                </Tag>
              ))}
              <TagInputField
                value={tagInput}
                onChange={(e) => setTagInput(e.target.value)}
                onKeyPress={handleTagKeyPress}
                placeholder="Add tag and press Enter"
              />
            </TagInput>
          </FormGroup>

          <ButtonGroup>
            <Button type="button" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" variant="primary">
              {task ? 'Update Task' : 'Create Task'}
            </Button>
          </ButtonGroup>
        </Form>
      </Modal>
    </Overlay>
  );
};

export default TaskEditor;