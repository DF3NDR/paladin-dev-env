# Service Layer

The Service Layer is responsible for implementing the core business logic of the system. It contains the services that define the movement of data objects and domain entities and operations of the system. The Service Layer is independent of the external interfaces and infrastructure and encapsulates the domain-specific logic of the system. 

## Components

The Service Layer is composed of the following components:
- **Transport**:  Responsible for the flow of domain entities the system
- **Transaction**: Responsible for the flow of data objects and messaging
  
## Transport Services
- ContentAnalysisService
- ContentDeliveryService
- ContentIngestionService
- TaskManagementServie
- JobOrchestrationService
- SubjectManagementService
- UserManagmentService
- QueueManagmentService
- SchedulerService

## Transaction Services
- EventHandlingService
- NotificationService